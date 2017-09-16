use std::str::FromStr;

use combine::{Parser, many1, token, eof, optional};
use combine::char::digit;

#[derive(Debug)]
pub enum Range {
    From(u32),
    To(u32),
    Between(u32, u32),
}

impl Range {
    pub fn contains(&self, n: u32) -> bool {
        use Range::*;
        match *self {
            From(a) => a <= n,
            To(b) => n <= b,
            Between(a, b) => a <= n && n <= b,
        }
    }
}

impl FromStr for Range {
    type Err = String;
    fn from_str(s: &str) -> Result<Range, String> {
        use Range::*;
        let num = || many1(digit()).map(|string: String| string.parse::<u32>().unwrap());

        let mut range = num()
            .and(optional(token('-').with(optional(num()))))
            .map(|(a, b)| match b {
                Some(Some(b)) => Between(a, b),
                Some(None) => From(a),
                None => Between(a, a),
            })
            .or(token('-').with(num()).map(|b| To(b)))
            .skip(eof());

        range
            .parse(s)
            .map_err(|_| "could not parse range".to_string())
            .map(|o| o.0)
    }
}
