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
        .after_help(
r#"LIST should be a comma-separated list of ranges. Each range should be of one of the following
forms:

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
        .value_of("exclude")
        .map(|m| {
            m.split(',')
                .map(|s| s.parse())
                .collect::<Result<Ranges>>()
                .chain_err(|| "invalid argument to -x/--exclude")
        })
        .unwrap_or(Ok(Ranges::new()))?;

    let opt_delim = matches
        .value_of("delimiter")
        .unwrap_or(" \t");

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();

    let mut columns = Vec::new();
    let mut backlog = Vec::new();
    let mut row = Vec::new();
    let mut measuring = true;
    for line in stdin.lock().lines() {
        let line = line?;
        split_line(&line, &mut row, opt_delim);
        if measuring {
            update_columns(
                &mut columns,
                &row[..],
                opt_include_cols.as_ref(),
                &opt_exclude_cols,
                opt_truncate.as_ref(),
            );
            backlog.push(row.clone());
            if backlog.len() >= opt_lines {
                measuring = false;
                for col in &mut columns {
                    col.calculate_size(opt_ratio);
                }
                for row in &backlog {
                    print_row(&mut stdout, &columns[..], row)?;
                }
                backlog.clear();
            }
        } else {
            print_row(&mut stdout, &columns[..], &row[..])?;
        }
        row.clear();
    }

    if measuring {
        for col in &mut columns {
            col.calculate_size(opt_ratio);
        }
    }
    for row in &backlog {
        print_row(&mut stdout, &columns[..], row)?;
    }
    Ok(())
}

enum State {
    Whitespace,
    NonWhitespace,
    EndDelim(char),
}

fn split_line(input: &str, row: &mut Vec<String>, delim: &str) {
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
                } else if !delim.contains(ch) {
                    start = Some(i);
                    state = NonWhitespace;
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
    }
}

fn update_columns(
    columns: &mut Vec<Column>,
    row: &[String],
    include_cols: Option<&Ranges>,
    excluded_cols: &Ranges,
    truncate_cols: Option<&Ranges>,
) {
    for i in 0..min(columns.len(), row.len()) {
        columns[i].add_sample(row[i].len());
    }
    for i in columns.len()..row.len() {
        let mut col = Column::new(row[i].len());
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
