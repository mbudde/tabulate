# tabulate

Align data in columns using heuristics to find suitable column size that does
not waste too much empty space. Useful for files like access logs where some
lines have columns that are much larger than than the rest of the lines.

```
$ cat <<EOF | tabulate
aaa bbb ccc
a   b   c
aaaaaaaaaaaaaaaaaaaaaaa bb cc
aaaaa b ccccc
aaa bb ccc
aaaa bb cccc
EOF
aaa    bbb  ccc
a      b    c
aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  bb  cc
aaaaa  b    ccccc
aaa    bb   ccc
aaaaa  b    ccccc
aaa    bb   ccc
aaa    bb   ccc
aaaaa  b    ccccc
aaa    bb   ccc
aaa    bbb  cccc
aaaa   bb   cccc
aaa    bb   ccc
aaaa   bb   cccc
```

```
tabulate

USAGE:
    tabulate [FLAGS] [OPTIONS]

FLAGS:
    -h, --help        Prints help information
    -t, --truncate    Truncate data that does not fit in a column
    -V, --version     Prints version information

OPTIONS:
    -c, --compress-cols <RATIO>
            Control how much columns are compressed (0 disabled column compression,
            default: 1.0)
    -n, --estimate-count <N>       Estimate column sizes from the first N lines
    -x, --exclude <LIST>
            Columns to hide (starts from 1; defaults to no columns)

    -i, --include <LIST>
            Columns to show (starts from 1, defaults to all columns)


LIST should be a comma-separated list of ranges. Each range should be of one of the
following forms:

  N       N'th column, starting at 1
  N-      from N'th column to end of line
  N-M     from N'th to M'th column
  -M      from first to M'th column
```
