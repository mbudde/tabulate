use clap::Parser;

use tabulate::{
    errors::*,
    range::Ranges,
    Options,
};

const BUILD_INFO: &str = include_str!(concat!(env!("OUT_DIR"), "/build-info.txt"));


#[derive(Clone)]
struct RangesValueParser;

impl clap::builder::TypedValueParser for RangesValueParser {
   type Value = Ranges;

   fn parse_ref(
       &self,
       cmd: &clap::Command,
       arg: Option<&clap::Arg>,
       value: &std::ffi::OsStr,
   ) -> std::result::Result<Self::Value, clap::Error> {
       let inner = clap::builder::StringValueParser::new();
       let val: String = inner.parse(cmd, arg, value.to_owned())?;

       let delimiter = arg.and_then(|a| a.get_value_delimiter()).unwrap_or(',');
       val.split(delimiter)
            .map(|s| s.parse())
            .collect::<Result<Ranges>>()
            .map_err(|_| {
                use clap::error::*;
                let mut err = Error::new(ErrorKind::ValueValidation)
                    .with_cmd(cmd);
                if let Some(arg) = arg {
                    err.insert(ContextKind::InvalidArg, ContextValue::String(arg.to_string()));
                }
                err.insert(ContextKind::InvalidValue, ContextValue::String(val));
                err
            })
   }
}

#[derive(Parser, Debug)]
#[command(author, about, long_about = None)]
#[command(version = format!("{}{}", env!("CARGO_PKG_VERSION"), BUILD_INFO))]
#[command(next_line_help = true, color = clap::ColorChoice::Never)]
#[command(after_help = r#"LIST should be a comma-separated list of ranges. Each range should be of one of the following forms:

  N       N'th column, starting at 1
  N-      from N'th column to end of line
  N-M     from N'th to M'th column
  -M      from first to M'th column"#)]
struct Args {
    /// Truncate data that does not fit in a column.
    /// Takes an optional list of columns that should be truncated.
    /// If no LIST is given all columns are truncated.
    #[arg(short = 't', long, value_name = "LIST", value_delimiter = ',', num_args = 0.., default_missing_value="1-", value_parser = RangesValueParser)]
    truncate: Option<Ranges>,

    /// Number between 0.0 and 1.0 that controls how much columns are compressed.
    /// Set to 0 to disable column compression, i.e. columns are sized to fit the largest value.
    #[arg(short = 'c', long = "compress-cols", value_name = "RATIO", num_args = 1, default_value = "1.0")]
    pub ratio: f64,

    /// Estimate column sizes from the first N lines. The value 0 means all lines.
    #[arg(short = 'n', long = "estimate-count", value_name = "N", num_args = 1, default_value_t = 1000)]
    pub lines: usize,

    /// Select which columns to include in the output.
    #[arg(short = 'i', long = "include", value_name = "LIST", value_delimiter = ',', num_args = 1.., value_parser = RangesValueParser)]
    pub include_cols: Option<Ranges>,

    /// Select which columns should be excluded from the output.
    /// This option takes precedence over --include.
    #[arg(short = 'x', long = "exclude", value_name = "LIST", value_delimiter = ',', num_args = 1.., value_parser = RangesValueParser)]
    pub exclude_cols: Option<Ranges>,

    /// Use characters of DELIM as column delimiters.
    #[arg(short = 'd', long = "delimiter", value_name = "DELIM", num_args = 1, default_value = " \t")]
    pub delim: String,

    /// Specify the delimiter to use to separate columns in the output.
    #[arg(short = 'o', long = "output-delimiter", value_name = "DELIM", num_args = 1, default_value = "  ")]
    pub output_delim: String,

    /// Parse columns as strictly being delimited by a single delimiter.
    #[arg(short = 's', long = "strict")]
    pub strict_delim: bool,

    /// Print lines during column size estimation phase.
    #[arg(long)]
    pub online: bool,

    /// Print information about the columns.
    #[arg(long = "column-info", conflicts_with = "online")]
    pub print_info: bool,
}

fn main() {
    match run() {
        Ok(..) => {}
        Err(Error::Io(ref e)) if e.kind() == std::io::ErrorKind::BrokenPipe => {}
        Err(ref e) => {
            eprintln!("{}", e);
            ::std::process::exit(1);
        }
    }
}

fn run() -> Result<()> {
    let args = Args::parse();
    dbg!(&args);

    let opts = Options {
        truncate: args.truncate,
        ratio: args.ratio,
        lines: args.lines,
        include_cols: args.include_cols,
        exclude_cols: args.exclude_cols.unwrap_or(Ranges::new()),
        delim: args.delim,
        output_delim: args.output_delim,
        strict_delim: args.strict_delim,
        print_info: args.print_info,
        online: args.online,
    };

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let stdin = stdin.lock();
    let stdout = stdout.lock();

    tabulate::process(stdin, stdout, &opts)
}
