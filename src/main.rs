extern crate clap;
extern crate combine;

use std::io::{self, Write, BufRead};
use std::cmp::{min, max};

use clap::{App, Arg};

use column::Column;
use range::Range;

mod column;
mod range;

fn main() {
    match run() {
        Ok(..) => {}
        Err(ref e) if e.kind() == std::io::ErrorKind::BrokenPipe => {}
        Err(ref e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn run() -> io::Result<()> {
    let matches = App::new("tabulate")
        .arg(Arg::from_usage(
            "-t, --truncate 'Truncate data that does not fit in a column'",
        ))
        .arg(Arg::from_usage(
            "-c, --compress-cols=[RATIO] 'Compress columns so more data fits on the screen'",
        ))
        .arg(Arg::from_usage(
            "-n, --estimate-count=[N] 'Estimate column sizes from the first N lines'",
        ))
        .arg(Arg::from_usage(
            "-i, --include=[COLS] 'Columns to show (starts from 0, defaults to all columns)'",
        ))
        .arg(Arg::from_usage(
            "-x, --exclude=[COLS] 'Columns to hide (starts from 0; defaults to no columns)'",
        ))
        .get_matches();

    let opt_truncate = matches.is_present("truncate");

    let opt_ratio = matches
        .value_of("compress-cols")
        .map(|m| {
            m.parse().map_err(|_| {
                "could not parse value as a floating number".to_string()
            })
        })
        .unwrap_or(Ok((1.0)))
        .unwrap();

    let opt_lines = matches
        .value_of("estimate-count")
        .map(|m| {
            m.parse().map_err(
                |_| "could not parse value as a number".to_string(),
            )
        })
        .unwrap_or(Ok((1000)))
        .unwrap();

    let opt_include_cols = matches
        .value_of("include")
        .map(|m| {
            m.split(',')
                .map(|s| {
                    s.parse().map_err(|_| {
                        "could not parse value as a list of ranges".to_string()
                    })
                })
                .collect::<Result<Vec<_>, _>>()
                .map(Some)
        })
        .unwrap_or(Ok(None))
        .unwrap();
    let opt_exclude_cols = matches
        .value_of("exclude")
        .map(|m| {
            m.split(',')
                .map(|s| {
                    s.parse().map_err(|_| {
                        "could not parse value as a list of ranges".to_string()
                    })
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .unwrap_or(Ok(vec![]))
        .unwrap();

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();

    let mut columns = Vec::new();
    let mut backlog = Vec::new();
    let mut row = Vec::new();
    let mut measuring = true;
    for line in stdin.lock().lines() {
        let line = line.unwrap();
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
            .map(|v| !v.iter().any(|r| r.contains(i as u32)))
            .unwrap_or(false)
        {
            col.set_excluded(true);
        }
        if excluded_cols.iter().any(|r| r.contains(i as u32)) {
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
