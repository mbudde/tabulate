use std::str::FromStr;
use std::iter::FromIterator;

use combine::{Parser, many1, token, eof, optional};
use combine::char::digit;

use errors::*;

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
    type Err = Error;
    fn from_str(s: &str) -> Result<Range> {
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
            .map_err(|_| ErrorKind::RangeParseError(s.to_string()).into())
            .map(|o| o.0)
            .and_then(|r| match r {
                From(0) | To(0) | Between(0, _) => Err(ErrorKind::ColumnsStartAtOne.into()),
                Between(a, b) if b < a => Err(
                    ErrorKind::InvalidDecreasingRange(s.to_string()).into(),
                ),
                _ => Ok(r),
            })
    }
}

pub struct Ranges(pub Vec<Range>);

impl Ranges {
    pub fn new() -> Ranges {
        Ranges(Vec::new())
    }

    pub fn any_contains(&self, n: u32) -> bool {
        self.0.iter().any(|r| r.contains(n))
    }
}

impl FromIterator<Range> for Ranges {
    fn from_iter<I: IntoIterator<Item=Range>>(iter: I) -> Self {
        Ranges(Vec::from_iter(iter))
    }
}
