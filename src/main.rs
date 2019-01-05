use clap::{App, AppSettings, Arg};

use tabulate::{
    Options,
    range::{Range, Ranges},
    errors::*,
};


const BUILD_INFO: &str = include_str!(concat!(env!("OUT_DIR"), "/build-info.txt"));

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
        .version(format!("{}{}", env!("CARGO_PKG_VERSION"), BUILD_INFO).as_str())
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
             .help("Estimate column sizes from the first N lines. The value 0 means all lines"))
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
             .conflicts_with("online")
             .help("Print information about the columns"))
        .arg(Arg::with_name("online")
             .long("online")
             .help("Print lines during column size estimation phase"))
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
        .unwrap_or_else(|| Ok(Ranges::new()))?;

    let opt_delim = matches
        .value_of("delimiter")
        .unwrap_or(" \t")
        .to_string();

    let opts = Options {
        truncate: opt_truncate,
        ratio: opt_ratio,
        lines: opt_lines,
        include_cols: opt_include_cols,
        exclude_cols: opt_exclude_cols,
        delim: opt_delim,
        strict_delim: matches.is_present("strict-delimiter"),
        print_info: matches.is_present("column-info"),
        online: matches.is_present("online"),
    };

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let stdin = stdin.lock();
    let stdout = stdout.lock();

    tabulate::process(stdin, stdout, &opts)
}
