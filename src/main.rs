extern crate clap;
extern crate combine;

use std::io::{self, Write, BufRead};
use std::cmp::{min, max};
use std::str::FromStr;

use clap::{App, Arg};
use combine::{Parser, many1, token, eof, optional};
use combine::char::digit;

#[derive(Debug)]
struct Column {
    samples: Vec<(usize, usize)>,
    size: Option<usize>,
    excluded: bool,
}

impl Column {
    fn new(initial: usize) -> Column {
        Column {
            samples: vec![(initial, 0)],
            size: None,
            excluded: false,
        }
    }

    fn size(&self) -> usize {
        self.size.expect("column size has not been calculated")
    }

    fn calculate_size(&mut self, ratio: f64) {
        assert!(self.samples.len() > 0);

        if ratio == 0. { // Optimization
            self.size = Some(self.samples.iter().map(|p| p.0).max().unwrap_or(0));
        }

        let n: usize = self.samples.iter().map(|p| p.1).sum();
        let min = self.samples.iter().map(|p| p.0).min().unwrap();
        let max = self.samples.iter().map(|p| p.0).max().unwrap();
        let spread = (0.7 + 20.0 / (1 + max - min) as f64).powi(2);
        let prob = self.samples.iter().map(|&(s, x)| (s, x as f64 / n as f64)).collect::<Vec<_>>();

        let mut best_score = std::f64::MAX;
        let mut best_size = max;
        for l in min .. max+1 {
            let waste: f64 = prob.iter().take_while(|&&(s, _)| s < l)
                .map(|&(s, p)| p * l.saturating_sub(s) as f64)
                .sum();
            let overflow: f64 = prob.iter().skip_while(|&&(s, _)| s <= l)
                .map(|&(s, p)| p * s.saturating_sub(l) as f64)
                .sum();

            let score = ratio * (1.0 + waste) + (1.0 + overflow).powi(2) * spread;

            if score < best_score {
                best_score = score;
                best_size = l;
            } else {
                break;
            }
        }
        self.size = Some(best_size);
    }

    fn update(&mut self, val: usize) {
        match self.samples.binary_search_by_key(&val, |t| t.0) {
            Ok(i) => self.samples[i].1 += 1,
            Err(i) => self.samples.insert(i, (val, 1)),
        }
    }
}

fn main() {
    match run() {
        Ok(..) => {}
        Err(ref e) if e.kind() == std::io::ErrorKind::BrokenPipe => {}
        Err(ref e) => {
            writeln!(std::io::stderr(), "Error: {}", e).unwrap();
            std::process::exit(1);
        }
    }
}

fn run() -> io::Result<()> {
    let matches = App::new("tabulate")
        .arg(Arg::from_usage("-t, --truncate 'Truncate data that does not fit in a column'"))
        .arg(Arg::from_usage("-c, --compress-cols=[RATIO] 'Compress columns so more data fits on the screen'"))
        .arg(Arg::from_usage("-n, --estimate-count=[N] 'Estimate column sizes from the first N lines'"))
        .arg(Arg::from_usage("-i, --include=[COLS] 'Columns to show (starts from 0, defaults to all columns)'"))
        .arg(Arg::from_usage("-x, --exclude=[COLS] 'Columns to hide (starts from 0; defaults to no columns)'"))
        .get_matches();

    let opt_truncate = matches.is_present("truncate");

    let opt_ratio = matches.value_of("compress-cols")
        .map(|m| m.parse().map_err(|_| "could not parse value as a floating number".to_string()))
        .unwrap_or(Ok((1.0)))
        .unwrap();

    let opt_lines = matches.value_of("estimate-count")
        .map(|m| m.parse().map_err(|_| "could not parse value as a number".to_string()))
        .unwrap_or(Ok((1000)))
        .unwrap();

    let opt_include_cols = matches.value_of("include")
        .map(|m| m.split(',').map(|s| s.parse().map_err(|_| "could not parse value as a list of ranges".to_string())).collect::<Result<Vec<_>, _>>().map(Some))
        .unwrap_or(Ok(None))
        .unwrap();
    let opt_exclude_cols = matches.value_of("exclude")
        .map(|m| m.split(',').map(|s| s.parse().map_err(|_| "could not parse value as a list of ranges".to_string())).collect::<Result<Vec<_>, _>>())
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
            update_columns(&mut columns, &row[..], opt_include_cols.as_ref(), &opt_exclude_cols[..]);
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
                        row.push(input[s..i+1].to_owned());
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

fn update_columns(columns: &mut Vec<Column>, row: &[String], include_cols: Option<&Vec<Range>>, excluded_cols: &[Range]) {
    for i in 0..min(columns.len(), row.len()) {
        columns[i].update(row[i].len());
    }
    for i in columns.len()..row.len() {
        let mut col = Column::new(row[i].len());
        if include_cols.map(|v| !v.iter().any(|r| r.contains(i as u32))).unwrap_or(false) {
            col.excluded = true;
        }
        if excluded_cols.iter().any(|r| r.contains(i as u32)) {
            col.excluded = true;
        }
        columns.push(col);
    }
}

fn print_row<W: Write>(out: &mut W, columns: &[Column], row: &[String], truncate: bool) -> io::Result<()> {
    let mut first = true;
    let mut goal: usize = 0;
    let mut used: usize = 0;
    for (cell, col) in row.iter().zip(columns.iter()).filter(|&(_, col)| !col.excluded) {
        if !first {
            write!(out, "  ")?;
        }
        first = false;
        let width = col.size();
        let out_width = width.saturating_sub(used.saturating_sub(goal));
        if truncate && cell.len() > out_width {
            write!(out, "{}…", &cell[0..out_width-1])?;
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

#[derive(Debug)]
enum Range {
    From(u32),
    To(u32),
    Between(u32, u32),
}

impl Range {
    fn contains(&self, n: u32) -> bool {
        use Range::*;
        match *self {
            From(a)       => a <= n,
            To(b)         =>           n <= b,
            Between(a, b) => a <= n && n <= b,
        }
    }
}

impl FromStr for Range {
    type Err = String;
    fn from_str(s: &str) -> Result<Range, String> {
        use Range::*;
        let num = || many1(digit())
            .map(|string: String| string.parse::<u32>().unwrap());

        let mut range = num().and(optional(token('-').with(optional(num()))))
            .map(|(a, b)| match b {
                Some(Some(b)) => Between(a, b),
                Some(None) => From(a),
                None    => Between(a, a),
            })
            .or(token('-').with(num()).map(|b| To(b)))
            .skip(eof());

        range.parse(s).map_err(|_| "could not parse range".to_string()).map(|o| o.0)
    }
}
