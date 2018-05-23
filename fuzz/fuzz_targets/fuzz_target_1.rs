#![no_main]
#[macro_use] extern crate libfuzzer_sys;
extern crate tabulate;

use std::io::BufReader;

use tabulate::{
    Options,
    range::Ranges,
};

fuzz_target!(|data: &[u8]| {
    let opts = Options {
        truncate: None,
        ratio: 1.0,
        lines: 1000,
        include_cols: None,
        exclude_cols: Ranges::new(),
        delim: " \t".to_string(),
        strict_delim: false,
        print_info: false,
    };

    let reader = BufReader::new(data);
    let output: Vec<u8> = Vec::new();

    tabulate::process(reader, output, opts).unwrap();
});
