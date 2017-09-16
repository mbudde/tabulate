extern crate clap;
extern crate combine;
#[macro_use]
extern crate error_chain;

use std::io::{self, Write, BufRead};
use std::cmp::{min, max};

use clap::{App, Arg};

mod column;
mod range;

mod errors {
    error_chain!{
        foreign_links {
            Io(::std::io::Error);
        }

        errors {
            RangeParseError(s: String) {
                display("could not parse '{}' as a range", s)
            }
            InvalidDecreasingRange(s: String) {
                display("invalid decreasing range: {}", s)
            }
            ColumnsStartAtOne {
                display("columns are numbered starting from 1")
            }
        }
    }
}

use column::Column;
use errors::*;
use range::Range;

fn main() {
    match run() {
        Ok(..) => {}
        Err(Error(ErrorKind::Io(ref e), _)) if e.kind() == std::io::ErrorKind::BrokenPipe => {}
        Err(ref e) => {
            use error_chain::ChainedError;
            eprintln!("{}", e.display_chain());
            ::std::process::exit(1);
        }
    }
}

fn run() -> Result<()> {
    let matches = App::new("tabulate")
        .arg(Arg::from_usage(
            "-t, --truncate \
            'Truncate data that does not fit in a column'",
        ))
        .arg(Arg::from_usage(
            "-c, --compress-cols=[RATIO] \
            'Control how much columns are compressed (0 disabled column compression, default: 1.0)'",
        ))
        .arg(Arg::from_usage(
            "-n, --estimate-count=[N] \
            'Estimate column sizes from the first N lines'",
        ))
        .arg(Arg::from_usage(
            "-i, --include=[LIST] \
            'Columns to show (starts from 1, defaults to all columns)'",
        ))
        .arg(Arg::from_usage(
            "-x, --exclude=[LIST] \
            'Columns to hide (starts from 1; defaults to no columns)'",
        ))
        .after_help(
r#"LIST should be a comma-separated list of ranges. Each range should be of one of the following
forms:

  N       N'th column, starting at 1
  N-      from N'th column to end of line
  N-M     from N'th to M'th column
  -M      from first to M'th column"#)
        .get_matches();

    let opt_truncate = matches.is_present("truncate");

    let opt_ratio = matches
        .value_of("compress-cols")
        .map(|m| {
            m.parse().chain_err(
                || "could not parse argument to -c/--compress-cols as a floating number",
            )
        })
        .unwrap_or(Ok((1.0)))?;

    let opt_lines = matches
        .value_of("estimate-count")
        .map(|m| {
            m.parse().chain_err(
                || "could not parse argument to -n/--estimate-count as a number",
            )
        })
        .unwrap_or(Ok((1000)))?;

    let opt_include_cols = matches
        .value_of("include")
        .map(|m| {
            m.split(',')
                .map(|s| s.parse())
                .collect::<Result<Vec<_>>>()
                .chain_err(|| "invalid argument to -i/--include")
                .map(Some)
        })
        .unwrap_or(Ok(None))?;

    let opt_exclude_cols = matches
        .value_of("exclude")
        .map(|m| {
            m.split(',')
                .map(|s| s.parse())
                .collect::<Result<Vec<_>>>()
                .chain_err(|| "invalid argument to -x/--exclude")
        })
        .unwrap_or(Ok(vec![]))?;

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();

    let mut columns = Vec::new();
    let mut backlog = Vec::new();
    let mut row = Vec::new();
    let mut measuring = true;
    for line in stdin.lock().lines() {
        let line = line?;
        split_line(&line, &mut row);
        if measuring {
            update_columns(
                &mut columns,
                &row[..],
                opt_include_cols.as_ref(),
                &opt_exclude_cols[..],
            );
            backlog.push(row.clone());
            if backlog.len() >= opt_lines {
                measuring = false;
                for col in &mut columns {
                    col.calculate_size(opt_ratio);
                }
                for row in &backlog {
                    print_row(&mut stdout, &columns[..], row, opt_truncate)?;
                }
                backlog.clear();
            }
        } else {
            print_row(&mut stdout, &columns[..], &row[..], opt_truncate)?;
        }
        row.clear();
    }

    if measuring {
        for col in &mut columns {
            col.calculate_size(opt_ratio);
        }
    }
    for row in &backlog {
        print_row(&mut stdout, &columns[..], row, opt_truncate)?;
    }
    Ok(())
}

enum State {
    Whitespace,
    NonWhitespace,
    EndDelim(char),
}

fn split_line(input: &str, row: &mut Vec<String>) {
    use State::*;

    let mut state = Whitespace;

    let mut start = None;
    let mut i = 0;
    let mut chars = input.chars();
    let mut current_char = chars.next();
    while let Some(ch) = current_char.take() {
        // print!("{} ", ch);
        match state {
            Whitespace => {
                // println!("whitespace");
                if ch == '(' || ch == '[' || ch == '"' {
                    let end_delim = match ch {
                        '(' => ')',
                        '[' => ']',
                        '"' => '"',
                        _ => unimplemented!(),
                    };
                    start = Some(i);
                    state = EndDelim(end_delim);
                } else if ch != ' ' && ch != '\t' {
                    start = Some(i);
                    state = NonWhitespace;
                }
            }
            NonWhitespace => {
                // println!("non-whitespace");
                if ch == ' ' || ch == '\t' {
                    if let Some(s) = start {
                        // println!("output = {:?}", &input[s..i]);
                        row.push(input[s..i].to_owned());
                    }
                    start = None;
                    state = Whitespace;
                }
            }
            EndDelim(delim) => {
                // println!("end-delim({})", delim);
                if ch == delim {
                    if let Some(s) = start {
                        // println!("output = {:?}", &input[s..i+1]);
                        row.push(input[s..i + 1].to_owned());
                    }
                    start = None;
                    state = Whitespace;
                }
            }
        }
        if current_char.is_none() {
            current_char = chars.next();
            i += 1;
        }
    }
    if let Some(s) = start {
        // println!("output = {:?}", &input[s..i]);
        row.push(input[s..i].to_owned());
    }
}

fn update_columns(
    columns: &mut Vec<Column>,
    row: &[String],
    include_cols: Option<&Vec<Range>>,
    excluded_cols: &[Range],
) {
    for i in 0..min(columns.len(), row.len()) {
        columns[i].add_sample(row[i].len());
    }
    for i in columns.len()..row.len() {
        let mut col = Column::new(row[i].len());
        if include_cols
            .map(|v| !v.iter().any(|r| r.contains((i + 1) as u32)))
            .unwrap_or(false)
        {
            col.set_excluded(true);
        }
        if excluded_cols.iter().any(|r| r.contains((i + 1) as u32)) {
            col.set_excluded(true);
        }
        columns.push(col);
    }
}

fn print_row<W: Write>(
    out: &mut W,
    columns: &[Column],
    row: &[String],
    truncate: bool,
) -> io::Result<()> {
    let mut first = true;
    let mut goal: usize = 0;
    let mut used: usize = 0;
    for (cell, col) in row.iter().zip(columns.iter()).filter(
        |&(_, col)| !col.is_excluded(),
    )
    {
        if !first {
            write!(out, "  ")?;
        }
        first = false;
        let width = col.size();
        let out_width = width.saturating_sub(used.saturating_sub(goal));
        if truncate && cell.len() > out_width {
            write!(out, "{}…", &cell[0..out_width - 1])?;
            used += out_width;
        } else {
            write!(out, "{:1$}", cell, out_width)?;
            used += max(out_width, cell.len());
        }
        goal += width;
    }
    write!(out, "\n")?;
    Ok(())
}
