use std::cmp::min;
use std::io::{self, BufRead, Write};

use crate::column::{Column, MeasureColumn};
use crate::errors::*;
use crate::parser::{Row, RowParser};
use crate::range::{Range, Ranges};

pub mod column;
pub mod parser;
pub mod range;
mod utils;

pub mod errors {
    pub type Result<T> = std::result::Result<T, Error>;

    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error("IO error")]
        Io(#[from] ::std::io::Error),

        #[error("could not parse '{}' as a range", .s)]
        RangeParseError {
            s: String
        },

        #[error("invalid decreasing range: {}", .s)]
        InvalidDecreasingRange {
            s: String
        },

        #[error("columns are numbered starting from 1")]
        ColumnsStartAtOne,
    }
}

#[derive(Debug)]
pub struct Options {
    pub truncate: Option<Ranges>,
    pub ratio: f64,
    pub lines: usize,
    pub include_cols: Option<Ranges>,
    pub exclude_cols: Ranges,
    pub delim: String,
    pub strict_delim: bool,
    pub print_info: bool,
    pub online: bool,
}

pub fn process<R: BufRead, W: Write>(input: R, mut output: W, opts: &Options) -> Result<()> {
    #[derive(Debug)]
    enum ProcessingState {
        Measuring {
            lines_measured: usize,
            backlog: Vec<Row>,
        },
        PrintBacklog {
            backlog: Vec<Row>,
        },
        ProcessInput,
    }

    let mut state = ProcessingState::Measuring {
        lines_measured: 1,
        backlog: Vec::new(),
    };
    let mut measure_columns = Vec::new();
    let mut columns = Vec::new();
    let parser = RowParser::new(opts.delim.clone(), opts.strict_delim);
    let mut row = Row::new();
    let mut lines = input.lines();

    loop {
        state = match state {
            ProcessingState::Measuring {
                lines_measured,
                mut backlog,
            } => {
                if let Some(line) = lines.next() {
                    let line = line?;
                    parser.parse_into(&mut row, line);
                    update_columns(
                        &mut measure_columns,
                        &row,
                        opts.include_cols.as_ref(),
                        &opts.exclude_cols,
                        opts.truncate.as_ref(),
                        opts.print_info,
                    );
                    if opts.online {
                        columns.clear();
                        columns
                            .extend(measure_columns.iter().map(|c| c.calculate_size(opts.ratio)));
                        print_row(&mut output, &columns[..], &row)?;
                    } else {
                        backlog.push(row.clone());
                    }
                    if opts.lines == 0 || lines_measured < opts.lines {
                        ProcessingState::Measuring {
                            lines_measured: lines_measured + 1,
                            backlog,
                        }
                    } else {
                        ProcessingState::PrintBacklog { backlog }
                    }
                } else {
                    ProcessingState::PrintBacklog { backlog }
                }
            }
            ProcessingState::PrintBacklog { backlog } => {
                columns.clear();
                columns.extend(measure_columns.iter().map(|c| c.calculate_size(opts.ratio)));

                if opts.print_info {
                    for (i, col) in columns.iter_mut().enumerate() {
                        writeln!(output, "Column {}", i + 1)?;
                        col.print_info(&mut output)?;
                        writeln!(output)?;
                    }
                    return Ok(());
                }

                for row in backlog {
                    print_row(&mut output, &columns[..], &row)?;
                }

                ProcessingState::ProcessInput
            }
            ProcessingState::ProcessInput => {
                if let Some(line) = lines.next() {
                    let line = line?;
                    parser.parse_into(&mut row, line);
                    print_row(&mut output, &columns[..], &row)?;

                    ProcessingState::ProcessInput
                } else {
                    break;
                }
            }
        }
    }

    Ok(())
}

fn update_columns(
    columns: &mut Vec<MeasureColumn>,
    row: &Row,
    include_cols: Option<&Ranges>,
    excluded_cols: &Ranges,
    truncate_cols: Option<&Ranges>,
    collect_info: bool,
) {
    for i in 0..min(columns.len(), row.len()) {
        columns[i].add_sample(&row[i]);
    }
    #[allow(clippy::needless_range_loop)]
    for i in columns.len()..row.len() {
        let mut col = MeasureColumn::new(collect_info);
        let col_num = (i + 1) as u32;

        let included = include_cols
            .map(|rs| rs.any_contains(col_num))
            .unwrap_or(true);

        let excluded = excluded_cols.any_contains(col_num);

        let truncated = truncate_cols
            .map(|rs| rs.any_contains(col_num))
            .unwrap_or(false);

        col.set_excluded(!included || excluded);
        col.set_truncated(truncated);

        col.add_sample(&row[i]);

        columns.push(col);
    }
}

fn print_row<W: Write>(out: &mut W, columns: &[Column], row: &Row) -> io::Result<()> {
    let mut overflow: usize = 0;
    for ((cell, col), first, last) in utils::first_last_iter(
        row.get_parts()
            .zip(columns)
            .filter(|&(_, col)| !col.is_excluded()),
    ) {
        if !first {
            write!(out, "  ")?;
        }
        overflow = col.print_cell(out, cell, overflow, last)?;
    }
    writeln!(out)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    #[test]
    fn basic_test() {
        let opts = Options {
            truncate: None,
            ratio: 1.0,
            lines: 1000,
            include_cols: None,
            exclude_cols: Ranges::new(),
            delim: " \t".to_string(),
            strict_delim: false,
            print_info: false,
            online: false,
        };

        let reader = BufReader::new(&b"aa bb cc\n1 2 3\n"[..]);
        let mut output: Vec<u8> = Vec::new();
        process(reader, &mut output, &opts).unwrap();
        assert_eq!(&output, b"aa  bb  cc\n1   2   3\n");
    }

    #[test]
    fn exclude_column() {
        let mut opts = Options {
            truncate: None,
            ratio: 1.0,
            lines: 1000,
            include_cols: None,
            exclude_cols: Ranges(vec![Range::Between(2, 2)]),
            delim: " \t".to_string(),
            strict_delim: false,
            print_info: false,
            online: false,
        };

        let input: &[u8] = b"aa bb cc\n1 2 3\n";
        let mut output: Vec<u8> = Vec::new();
        process(BufReader::new(input), &mut output, &opts).unwrap();
        assert_eq!(&output, b"aa  cc\n1   3\n");

        opts.exclude_cols = Ranges(vec![Range::From(2)]);
        output.clear();
        process(BufReader::new(input), &mut output, &opts).unwrap();
        assert_eq!(&output, b"aa\n1\n");

        opts.exclude_cols = Ranges(vec![Range::To(2)]);
        output.clear();
        process(BufReader::new(input), &mut output, &opts).unwrap();
        assert_eq!(&output, b"cc\n3\n");

        opts.exclude_cols = Ranges(vec![Range::Between(1, 1), Range::Between(3, 3)]);
        output.clear();
        process(BufReader::new(input), &mut output, &opts).unwrap();
        assert_eq!(&output, b"bb\n2\n");
    }

    #[test]
    fn lines_opt() {
        let opts = Options {
            truncate: None,
            ratio: 1.0,
            lines: 1,
            include_cols: None,
            exclude_cols: Ranges::new(),
            delim: " \t".to_string(),
            strict_delim: false,
            print_info: false,
            online: false,
        };

        let reader = BufReader::new(&b"1 1\naaaa aaaa\n"[..]);
        let mut output: Vec<u8> = Vec::new();
        process(reader, &mut output, &opts).unwrap();
        assert_eq!(&output, b"1  1\naaaa  aaaa\n");
    }

    #[test]
    fn overflow() {
        let opts = Options {
            truncate: None,
            ratio: 1.0,
            lines: 1,
            include_cols: None,
            exclude_cols: Ranges::new(),
            delim: " \t".to_string(),
            strict_delim: false,
            print_info: false,
            online: false,
        };

        // a  a  aaaaaaaaaaa  a
        // a  a  aaaaaaaaaaa  a
        // a  a  aaaaaaaaaaa  a
        // bbbbbb  bb  b      b
        let input = ("a a aaaaaaaaaaa a\n".repeat(10) + "bbbbbb bb b b\n").into_bytes();
        let expected = "a  a  aaaaaaaaaaa  a\n".repeat(10) + "bbbbbb  bb  b      b\n";
        let reader = BufReader::new(&input[..]);
        let mut output: Vec<u8> = Vec::new();
        process(reader, &mut output, &opts).unwrap();
        assert_eq!(std::str::from_utf8(&output).unwrap(), expected);
    }
}
