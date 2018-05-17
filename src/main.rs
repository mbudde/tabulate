extern crate clap;
extern crate combine;
#[macro_use]
extern crate error_chain;

use std::io::{self, Write, BufRead};
use std::cmp::{min, max};

use clap::{App, AppSettings, Arg};

mod column;
mod range;

const BUILD_INFO: &'static str = include_str!(concat!(env!("OUT_DIR"), "/build-info.txt"));

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

use column::{Column, MeasureColumn};
use errors::*;
use range::{Range, Ranges};

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
        .version(format!("{} ({})", env!("CARGO_PKG_VERSION"), BUILD_INFO).as_str())
        .setting(AppSettings::UnifiedHelpMessage)
        .setting(AppSettings::NextLineHelp)
        .arg(Arg::with_name("truncate")
             .short("t").long("truncate")
             .value_name("LIST")
             .require_delimiter(true)
             .min_values(0)
             .help("Truncate data that does not fit in a column. \
                   Takes an optional list of columns that should be truncated. \
                   If no LIST is given all columns are truncated"))
        .arg(Arg::with_name("compress-cols")
             .short("c").long("compress-cols")
             .value_name("RATIO")
             .number_of_values(1)
             .default_value("1.0")
             .help("Control how much columns are compressed. \
                   Set to 0 to disable column compression, i.e. columns are sized to fit \
                   the largest value"))
        .arg(Arg::with_name("estimate-count")
             .short("n").long("estimate-count")
             .value_name("N")
             .number_of_values(1)
             .default_value("1000")
             .help("Estimate column sizes from the first N lines"))
        .arg(Arg::with_name("include")
             .short("i").long("include")
             .value_name("LIST")
             .use_delimiter(true)
             .min_values(1)
             .help("Select which columns to include in the output"))
        .arg(Arg::with_name("exclude")
             .short("x").long("exclude")
             .value_name("LIST")
             .require_delimiter(true)
             .min_values(1)
             .help("Select which columns should be excluded from the output. \
                   This option takes precedence over --include"))
        .arg(Arg::with_name("delimiter")
             .short("d").long("delimiter")
             .value_name("DELIM")
             .number_of_values(1)
             .help("Use characters of DELIM as column delimiters [default: \" \\t\"]"))
        .arg(Arg::with_name("strict-delimiter")
             .short("s").long("strict")
             .help("Parse columns as strictly being delimited by a single delimiter"))
        .arg(Arg::with_name("column-info")
             .long("column-info")
             .help("Print information about the columns"))
        .after_help(
r#"LIST should be a comma-separated list of ranges. Each range should be of one of the following forms:

  N       N'th column, starting at 1
  N-      from N'th column to end of line
  N-M     from N'th to M'th column
  -M      from first to M'th column"#)
        .get_matches();

    let opt_truncate = matches
        .values_of("truncate")
        .map(|m| {
            m.map(|s| s.parse())
                .collect::<Result<Ranges>>()
                .chain_err(|| "invalid argument to -t/--truncate")
                .map(|mut v| {
                    if v.0.is_empty() {
                        v.0.push(Range::From(1));
                    }
                    Some(v)
                })
        })
        .unwrap_or(Ok(None))?;

    let opt_ratio = matches
        .value_of("compress-cols").unwrap()
        .parse().chain_err(
            || "could not parse argument to -c/--compress-cols as a floating number",
        )?;

    let opt_lines = matches
        .value_of("estimate-count").unwrap()
        .parse().chain_err(
            || "could not parse argument to -n/--estimate-count as a number",
        )?;

    let opt_include_cols = matches
        .values_of("include")
        .map(|m| {
            m.map(|s| s.parse())
                .collect::<Result<Ranges>>()
                .chain_err(|| "invalid argument to -i/--include")
                .map(Some)
        })
        .unwrap_or(Ok(None))?;

    let opt_exclude_cols = matches
        .values_of("exclude")
        .map(|m| {
            m.map(|s| s.parse())
                .collect::<Result<Ranges>>()
                .chain_err(|| "invalid argument to -x/--exclude")
        })
        .unwrap_or(Ok(Ranges::new()))?;

    let opt_delim = matches
        .value_of("delimiter")
        .unwrap_or(" \t");
    let opt_strict_delim = matches
        .is_present("strict-delimiter");

    let opt_print_info = matches.is_present("column-info");

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();

    #[derive(Debug)]
    enum ProcessingState {
        Measuring { backlog: Vec<Vec<String>> },
        PrintBacklog { backlog: Vec<Vec<String>> },
        ProcessInput,
    }

    let mut state = ProcessingState::Measuring { backlog: Vec::new() };
    let mut measure_columns = Vec::new();
    let mut columns = Vec::new();
    let mut row = Vec::new();
    let mut lines = stdin.lock().lines();

    loop {
        state = match state {
            ProcessingState::Measuring { mut backlog } => {
                if let Some(line) = lines.next() {
                    let line = line?;
                    split_line(&line, &mut row, opt_delim, opt_strict_delim);
                    update_columns(
                        &mut measure_columns,
                        &row[..],
                        opt_include_cols.as_ref(),
                        &opt_exclude_cols,
                        opt_truncate.as_ref(),
                        opt_print_info,
                    );
                    backlog.push(row.clone());
                    row.clear();
                    if backlog.len() >= opt_lines {
                        ProcessingState::PrintBacklog { backlog }
                    } else {
                        ProcessingState::Measuring { backlog }
                    }
                } else {
                    ProcessingState::PrintBacklog { backlog }
                }
            }
            ProcessingState::PrintBacklog { backlog } => {
                columns.extend(measure_columns.drain(..).map(|c| c.calculate_size(opt_ratio)));
                if opt_print_info {
                    for (i, col) in columns.iter_mut().enumerate() {
                        write!(stdout, "Column {}\n", i + 1)?;
                        col.print_info(&mut stdout)?;
                        write!(stdout, "\n")?;
                    }
                    return Ok(());
                }

                for row in &backlog {
                    print_row(&mut stdout, &columns[..], row)?;
                }

                ProcessingState::ProcessInput
            }
            ProcessingState::ProcessInput => {
                if let Some(line) = lines.next() {
                    let line = line?;
                    split_line(&line, &mut row, opt_delim, opt_strict_delim);
                    print_row(&mut stdout, &columns[..], &row[..])?;
                    row.clear();

                    ProcessingState::ProcessInput
                } else {
                    break;
                }
            }
        }
    }

    Ok(())
}

#[derive(Eq, PartialEq)]
enum State {
    Whitespace,
    NonWhitespace,
    EndDelim(char),
}

fn split_line(input: &str, row: &mut Vec<String>, delim: &str, strict_delim: bool) {
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
                if !strict_delim && (ch == '(' || ch == '[' || ch == '"') {
                    let end_delim = match ch {
                        '(' => ')',
                        '[' => ']',
                        '"' => '"',
                        _ => unimplemented!(),
                    };
                    start = Some(i);
                    state = EndDelim(end_delim);
                } else if !delim.contains(ch) {
                    start = Some(i);
                    state = NonWhitespace;
                } else if strict_delim {
                    row.push("".to_owned());
                }
            }
            NonWhitespace => {
                // println!("non-whitespace");
                if delim.contains(ch) {
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
    } else if strict_delim && state == Whitespace {
        row.push("".to_owned());
    }
}

fn update_columns(
    columns: &mut Vec<MeasureColumn>,
    row: &[String],
    include_cols: Option<&Ranges>,
    excluded_cols: &Ranges,
    truncate_cols: Option<&Ranges>,
    collect_info: bool,
) {
    for i in 0..min(columns.len(), row.len()) {
        columns[i].add_sample(&row[i]);
    }
    for i in columns.len()..row.len() {
        let mut col = MeasureColumn::new(row[i].len(), collect_info);
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

        columns.push(col);
    }
}

fn print_row<W: Write>(
    out: &mut W,
    columns: &[Column],
    row: &[String],
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
        if col.is_truncated() && cell.len() > out_width {
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


#[cfg(test)]
mod tests {
    use super::split_line;

    macro_rules! assert_vec {
        ($x:ident, [ $( $y:expr ),* ]) => {
           assert_eq!(&$x[..], &[$( $y.to_owned() ),*]);
        };
    }

    #[test]
    fn test_split_line_simple() {
        let mut row = Vec::new();
        split_line("a b c", &mut row, " ", false);
        assert_vec!(row, ["a", "b", "c"]);
    }

    #[test]
    fn test_split_line_collapse() {
        let mut row = Vec::new();
        split_line("a   b    c", &mut row, " ", false);
        assert_vec!(row, ["a", "b", "c"]);
    }

    #[test]
    fn test_split_line_ignore_leading_and_trailing() {
        let mut row = Vec::new();
        split_line("   a   b    c   ", &mut row, " ", false);
        assert_vec!(row, ["a", "b", "c"]);
    }

    #[test]
    fn test_split_line_empty() {
        let mut row = Vec::new();
        split_line("", &mut row, " ", false);
        assert!(row.is_empty());

        row.clear();
        split_line(" ", &mut row, " ", false);
        assert!(row.is_empty());
    }

    #[test]
    fn test_split_line_strict() {
        let mut row = Vec::new();
        split_line("a b c", &mut row, " ", true);
        assert_vec!(row, ["a", "b", "c"]);

        row.clear();
        split_line(" a b  c", &mut row, " ", true);
        assert_vec!(row, ["", "a", "b", "", "c"]);
    }

    #[test]
    fn test_split_line_strict_trailing_whitespace() {
        let mut row = Vec::new();
        split_line("a ", &mut row, " ", true);
        assert_vec!(row, ["a", ""]);

        row.clear();
        split_line("a  ", &mut row, " ", true);
        assert_vec!(row, ["a", "", ""]);
    }

    #[test]
    fn test_split_line_strict_empty() {
        let mut row = Vec::new();
        split_line("", &mut row, " ", true);
        assert_vec!(row, [""]);

        row.clear();
        split_line(" ", &mut row, " ", true);
        assert_vec!(row, ["", ""]);
    }
}
