# tabulate

Align data in columns using heuristics to find suitable column sizes that
minimize the amount of wasted space. Useful for files like access logs where
some lines have columns that are much larger than than the rest of the lines.

```
$ cat input.txt
aaa bbb ccc
a   b   c
aaaaaaaaaaaaaaaaaaaaaaa bb cc
aaaaa b ccccc
aaa bb ccc
aaaa bb cccc
aaa bb ccc
aaa bb ccc
aaaaa b ccccc
aaa bb ccc
aaa bb ccc
aaaaa b ccccc
$ tabulate <input.txt
aaa       bbb  ccc
a         b    c
aaaaaaaaaaaaaaaaaaaaaaa  bb  cc
aaaaa     b    ccccc
aaa       bb   ccc
aaaa      bb   cccc
aaa       bb   ccc
aaa       bb   ccc
aaaaa     b    ccccc
aaa       bb   ccc
aaa       bb   ccc
aaaaa     b    ccccc
$ tabulate -t <input.txt
aaa       bbb  ccc
a         b    c
aaaaaaa…  bb   cc
aaaaa     b    ccccc
aaa       bb   ccc
aaaa      bb   cccc
aaa       bb   ccc
aaa       bb   ccc
aaaaa     b    ccccc
aaa       bb   ccc
aaa       bb   ccc
aaaaa     b    ccccc
```

## Installing

```
cargo install tabulate
```

## Options

```
tabulate

USAGE:
    tabulate [OPTIONS]

FLAGS:
        --column-info    Print information about the columns
    -h, --help           Prints help information
    -V, --version        Prints version information

OPTIONS:
    -c, --compress-cols <RATIO>
            Control how much columns are compressed. Set to 0 to disable column
            compression [default: 1.0]
    -d, --delimiter <DELIM>
            Use characters of DELIM as column delimiters [default: " \t"]

    -n, --estimate-count <N>
            Estimate column sizes from the first N lines [default: 1000]

    -x, --exclude <LIST>
            Select which columns should be excluded from the output. This option
            takes precedence over --include
    -i, --include <LIST>           Select which columns to include in the output
    -t, --truncate <LIST>
            Truncate data that does not fit in a column. Takes an optional list of
            columns that should be truncated. If no LIST is given all columns are
            truncated

LIST should be a comma-separated list of ranges. Each range should be of one of the
following forms:

  N       N'th column, starting at 1
  N-      from N'th column to end of line
  N-M     from N'th to M'th column
  -M      from first to M'th column
```
