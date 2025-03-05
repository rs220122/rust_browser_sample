use alloc::{string::String, vec::Vec};

#[derive(Debug, Clone, PartialEq)]
pub enum CssToken {
    HashToken(String),
    Delim(char),
    Number(f64),
    Colon,
    SemiColon,
    OpenParenthesis,
    CloseParenthesis,
    OpenCurly,
    CloseCurly,
    // 識別子トークン
    Ident(String),
    StringToken(String),
    AtKeyword(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CssTokenizer {
    pos: usize,
    input: Vec<char>,
}

impl CssTokenizer {
    pub fn new(css: String) -> Self {
        Self {
            pos: 0,
            input: css.chars().collect(),
        }
    }

    fn consume_string_token(&mut self) -> String {
        let mut s = String::new();

        loop {
            if self.pos >= self.input.len() {
                return s;
            }

            self.pos += 1;
            let c = self.input[self.pos];
            match c {
                '"' | '\'' => break,
                _ => s.push(c),
            }
        }
        s
    }

    fn consume_numeric_token(&mut self) -> f64 {
        let mut num = 0f64;
        let mut floating = false;
        let mut floating_digit = 1f64;

        // 数字　or ピリオドが車で取得し続ける
        loop {
            if self.pos >= self.input.len() {
                return num;
            }

            let c = self.input[self.pos];
            match c {
                '0'..='9' => {
                    if floating {
                        // 小数点の後は、値を1/10する.
                        floating_digit += 1f64 / 10f64;
                        num += (c.to_digit(10).unwrap() as f64) * floating_digit;
                    } else {
                        num = num * 10.0 + (c.to_digit(10).unwrap() as f64);
                    }
                    self.pos += 1;
                }
                '.' => {
                    floating = true;
                    self.pos += 1;
                }
                _ => break,
            }
        }
        num
    }

    fn consume_ident_token(&mut self) -> String {
        let mut s = String::new();
        s.push(self.input[self.pos]);
        loop {
            if self.pos >= self.input.len() {
                return s;
            }
            self.pos += 1;
            let c = self.input[self.pos];
            match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' => {
                    s.push(c);
                }
                _ => break,
            }
        }
        s
    }
}

impl Iterator for CssTokenizer {
    type Item = CssToken;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.pos >= self.input.len() {
                return None;
            }

            let c = self.input[self.pos];

            let token = match c {
                '(' => CssToken::OpenParenthesis,
                ')' => CssToken::CloseParenthesis,
                ',' => CssToken::Delim(','),
                '.' => CssToken::Delim('.'),
                ':' => CssToken::Colon,
                ';' => CssToken::SemiColon,
                '{' => CssToken::OpenCurly,
                '}' => CssToken::CloseCurly,
                ' ' | '\n' => {
                    self.pos += 1;
                    continue;
                }
                '"' | '\'' => {
                    let value = self.consume_string_token();
                    CssToken::StringToken(value)
                }
                '0'..='9' => {
                    let t = CssToken::Number(self.consume_numeric_token());
                    self.pos -= 1;
                    t
                }

                // 識別子トークンを返す
                '#' => {
                    let value = self.consume_ident_token();
                    self.pos -= 1;
                    CssToken::HashToken(value)
                }
                '-' => {
                    let value = self.consume_ident_token();
                    self.pos -= 1;
                    CssToken::Ident(value)
                }
                '@' => {
                    // 次の3文字が識別子として有効な文字の場合、<at-keyword-token>
                    if self.input.len() >= self.pos + 3
                        && self.input[self.pos + 1].is_ascii_alphabetic()
                        && self.input[self.pos + 2].is_ascii_alphabetic()
                        && self.input[self.pos + 3].is_ascii_alphabetic()
                    {
                        // skip '@'
                        self.pos += 1;
                        let t = CssToken::AtKeyword(self.consume_ident_token());
                        self.pos -= 1;
                        t
                    } else {
                        CssToken::Delim('@')
                    }
                }
                'a'..='z' | 'A'..='Z' | '_' => {
                    let t = CssToken::Ident(self.consume_ident_token());
                    self.pos -= 1;
                    t
                }
                _ => unimplemented!("char {} is not supported yet", c),
            };

            self.pos += 1;
            return Some(token);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn test_empty() {
        let s = "".to_string();
        let mut t = CssTokenizer::new(s);
        assert!(t.next().is_none());
    }

    #[test]
    fn test_one_rule() {
        let style = "p {background-color: red; }".to_string();
        let mut t = CssTokenizer::new(style);
        let expected = [
            Some(CssToken::Ident("p".to_string())),
            Some(CssToken::OpenCurly),
            Some(CssToken::Ident("background-color".to_string())),
            Some(CssToken::Colon),
            Some(CssToken::Ident("red".to_string())),
            Some(CssToken::SemiColon),
            Some(CssToken::CloseCurly),
            None,
        ];

        for e in expected {
            assert_eq!(e, t.next());
        }
    }

    #[test]
    fn test_id_selector() {
        let style = "#test {   color: red; }".to_string();
        let mut t = CssTokenizer::new(style);
        let expected = [
            CssToken::HashToken("#test".to_string()),
            CssToken::OpenCurly,
            CssToken::Ident("color".to_string()),
            CssToken::Colon,
            CssToken::Ident("red".to_string()),
            CssToken::SemiColon,
            CssToken::CloseCurly,
        ];

        for e in expected {
            assert_eq!(e, t.next().expect("failed"));
        }
        assert!(t.next().is_none());
    }

    #[test]
    fn test_class_selector() {
        let style = ".test_class { color: red; }".to_string();
        let mut t = CssTokenizer::new(style);
        let expected = [
            CssToken::Delim('.'),
            CssToken::Ident("test_class".to_string()),
            CssToken::OpenCurly,
            CssToken::Ident("color".to_string()),
            CssToken::Colon,
            CssToken::Ident("red".to_string()),
            CssToken::SemiColon,
            CssToken::CloseCurly,
        ];

        for e in expected {
            assert_eq!(e, t.next().expect("failed"));
        }
        assert!(t.next().is_none());
    }

    #[test]
    fn test_multiple_rules() {
        let style =
            "p {content: \"Test\"; } h1 { font-size: 10px; color: blue;}"
                .to_string();
        let mut t = CssTokenizer::new(style);
        let expected = [
            CssToken::Ident("p".to_string()),
            CssToken::OpenCurly,
            CssToken::Ident("content".to_string()),
            CssToken::Colon,
            CssToken::StringToken("Test".to_string()),
            CssToken::SemiColon,
            CssToken::CloseCurly,
            CssToken::Ident("h1".to_string()),
            CssToken::OpenCurly,
            CssToken::Ident("font-size".to_string()),
            CssToken::Colon,
            CssToken::Number(10f64),
            CssToken::Ident("px".to_string()),
            CssToken::SemiColon,
            CssToken::Ident("color".to_string()),
            CssToken::Colon,
            CssToken::Ident("blue".to_string()),
            CssToken::SemiColon,
            CssToken::CloseCurly,
        ];

        for e in expected {
            assert_eq!(Some(e), t.next());
        }
        assert!(t.next().is_none());
    }

    #[test]
    fn test_atmark() {
        let style =
            "@media (max-width: 600px) body{background-color: lightblue;}"
                .to_string();
        let mut t = CssTokenizer::new(style);
        let expected = [
            CssToken::AtKeyword("media".to_string()),
            CssToken::OpenParenthesis,
            CssToken::Ident("max-width".to_string()),
            CssToken::Colon,
            CssToken::Number(600.0),
            CssToken::Ident("px".to_string()),
            CssToken::CloseParenthesis,
            CssToken::Ident("body".to_string()),
            CssToken::OpenCurly,
            CssToken::Ident("background-color".to_string()),
            CssToken::Colon,
            CssToken::Ident("lightblue".to_string()),
            CssToken::SemiColon,
            CssToken::CloseCurly,
        ];
        for e in expected {
            assert_eq!(Some(e), t.next());
        }
        assert!(t.next().is_none());
    }
}
