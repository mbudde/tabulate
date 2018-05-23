
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
}

impl ::std::ops::Index<usize> for Row {
    type Output = str;

    fn index(&self, index: usize) -> &str {
        let (i, j) = self.parts[index];
        &self.line[i..j]
    }
}


#[derive(Eq, PartialEq)]
enum ParseState {
    Whitespace,
    NonWhitespace,
    EndDelim(char),
}

pub struct RowParser {
    delim: String,
    strict_delim: bool
}

impl RowParser {
    pub fn new<S: Into<String>>(delim: S, strict_delim: bool) -> RowParser {
        RowParser {
            delim: delim.into(),
            strict_delim,
        }
    }

    pub fn parse_into<S: Into<String>>(&self, row: &mut Row, line: S) {
        use self::ParseState::*;

        row.line = line.into();
        row.parts.clear();

        let mut state = Whitespace;

        let mut start = None;
        let mut i = 0;
        let mut chars = row.line.chars();
        let mut current_char = chars.next();
        while let Some(ch) = current_char.take() {
            // print!("{} ", ch);
            match state {
                Whitespace => {
                    // println!("whitespace");
                    if !self.strict_delim && (ch == '(' || ch == '[' || ch == '"') {
                        let end_delim = match ch {
                            '(' => ')',
                            '[' => ']',
                            '"' => '"',
                            _ => unimplemented!(),
                        };
                        start = Some(i);
                        state = EndDelim(end_delim);
                    } else if !self.delim.contains(ch) {
                        start = Some(i);
                        state = NonWhitespace;
                    } else if self.strict_delim {
                        row.parts.push((i, i));
                    }
                }
                NonWhitespace => {
                    // println!("non-whitespace");
                    if self.delim.contains(ch) {
                        if let Some(s) = start {
                            // println!("output = {:?}", &input[s..i]);
                            row.parts.push((s, i));
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
                            row.parts.push((s, i + 1));
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
            row.parts.push((s, i));
        } else if self.strict_delim && state == Whitespace {
            row.parts.push((i, i));
        }
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
        let parser = RowParser::new(" ", false);
        let mut row = Row::new();
        parser.parse_into(&mut row, "a b c");
        assert_row!(row, ["a", "b", "c"]);
    }

    #[test]
    fn test_split_line_collapse() {
        let parser = RowParser::new(" ", false);
        let mut row = Row::new();
        parser.parse_into(&mut row, "a   b    c");
        assert_row!(row, ["a", "b", "c"]);
    }

    #[test]
    fn test_split_line_ignore_leading_and_trailing() {
        let parser = RowParser::new(" ", false);
        let mut row = Row::new();
        parser.parse_into(&mut row, "   a   b    c   ");
        assert_row!(row, ["a", "b", "c"]);
    }

    #[test]
    fn test_split_line_empty() {
        let parser = RowParser::new(" ", false);
        let mut row = Row::new();
        parser.parse_into(&mut row, "");
        assert!(row.get_parts().next().is_none());

        parser.parse_into(&mut row, " ");
        assert!(row.get_parts().next().is_none());
    }

    #[test]
    fn test_split_line_strict() {
        let parser = RowParser::new(" ", true);
        let mut row = Row::new();
        parser.parse_into(&mut row, "a b c");
        assert_row!(row, ["a", "b", "c"]);

        parser.parse_into(&mut row, " a b  c");
        assert_row!(row, ["", "a", "b", "", "c"]);
    }

    #[test]
    fn test_split_line_strict_trailing_whitespace() {
        let parser = RowParser::new(" ", true);
        let mut row = Row::new();
        parser.parse_into(&mut row, "a ");
        assert_row!(row, ["a", ""]);

        parser.parse_into(&mut row, "a  ");
        assert_row!(row, ["a", "", ""]);
    }

    #[test]
    fn test_split_line_strict_empty() {
        let parser = RowParser::new(" ", true);
        let mut row = Row::new();
        parser.parse_into(&mut row, "");
        assert_row!(row, [""]);

        parser.parse_into(&mut row, " ");
        assert_row!(row, ["", ""]);
    }
}