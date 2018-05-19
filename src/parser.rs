#[derive(Eq, PartialEq)]
enum ParseState {
    Whitespace,
    NonWhitespace,
    EndDelim(char),
}


#[derive(Clone, Debug)]
pub struct Row {
    parts: Vec<(usize, usize)>,
    line: String,
}

impl Row {
    pub fn new() -> Row {
        Row {
            parts: Vec::new(),
            line: String::new(),
        }
    }

    pub fn get_parts(&self) -> impl Iterator<Item=&str> {
        self.parts.iter().map(move |&(i, j)| &self.line[i..j])
    }

    pub fn len(&self) -> usize {
        self.parts.len()
    }

    pub fn parse<S: Into<String>>(&mut self, line: S, delim: &str, strict_delim: bool) {
        use self::ParseState::*;

        self.line = line.into();
        self.parts.clear();

        let mut state = Whitespace;

        let mut start = None;
        let mut i = 0;
        let mut chars = self.line.chars();
        let mut current_char = chars.next();
        while let Some(ch) = current_char.take() {
            // print!("{} ", ch);
            match state {
                Whitespace => {
                    // println!("whitespace");
                    if !strict_delim && (ch == '(' || ch == '[' || ch == '"') {
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
                    } else if strict_delim {
                        self.parts.push((i, i));
                    }
                }
                NonWhitespace => {
                    // println!("non-whitespace");
                    if delim.contains(ch) {
                        if let Some(s) = start {
                            // println!("output = {:?}", &input[s..i]);
                            self.parts.push((s, i));
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
                            self.parts.push((s, i + 1));
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
            self.parts.push((s, i));
        } else if strict_delim && state == Whitespace {
            self.parts.push((i, i));
        }
    }
}

impl ::std::ops::Index<usize> for Row {
    type Output = str;

    fn index(&self, index: usize) -> &str {
        let (i, j) = self.parts[index];
        &self.line[i..j]
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_row {
        ($x:ident, [ $( $y:expr ),* ]) => {
           assert_eq!(&$x.get_parts().collect::<Vec<_>>()[..], &[$( $y ),*]);
        };
    }

    #[test]
    fn test_split_line_simple() {
        let mut row = Row::new();
        row.parse("a b c", " ", false);
        assert_row!(row, ["a", "b", "c"]);
    }

    #[test]
    fn test_split_line_collapse() {
        let mut row = Row::new();
        row.parse("a   b    c", " ", false);
        assert_row!(row, ["a", "b", "c"]);
    }

    #[test]
    fn test_split_line_ignore_leading_and_trailing() {
        let mut row = Row::new();
        row.parse("   a   b    c   ", " ", false);
        assert_row!(row, ["a", "b", "c"]);
    }

    #[test]
    fn test_split_line_empty() {
        let mut row = Row::new();
        row.parse("", " ", false);
        assert!(row.get_parts().next().is_none());

        row.parse(" ", " ", false);
        assert!(row.get_parts().next().is_none());
    }

    #[test]
    fn test_split_line_strict() {
        let mut row = Row::new();
        row.parse("a b c", " ", true);
        assert_row!(row, ["a", "b", "c"]);

        row.parse(" a b  c", " ", true);
        assert_row!(row, ["", "a", "b", "", "c"]);
    }

    #[test]
    fn test_split_line_strict_trailing_whitespace() {
        let mut row = Row::new();
        row.parse("a ", " ", true);
        assert_row!(row, ["a", ""]);

        row.parse("a  ", " ", true);
        assert_row!(row, ["a", "", ""]);
    }

    #[test]
    fn test_split_line_strict_empty() {
        let mut row = Row::new();
        row.parse("", " ", true);
        assert_row!(row, [""]);

        row.parse(" ", " ", true);
        assert_row!(row, ["", ""]);
    }
}
