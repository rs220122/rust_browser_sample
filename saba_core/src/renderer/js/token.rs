use alloc::string::ToString;
use alloc::{string::String, vec::Vec};

// 予約後の定義
static RESERVED_WORDS: [&str; 3] = ["var", "function", "return"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Punctuator(char),
    Number(u64),
    // 変数を表す
    Identifier(String),
    // return, varなどの予約語を表す
    Keyword(String),
    StringLiteral(String),
}

pub struct JsLexer {
    pos: usize,
    input: Vec<char>,
}

impl JsLexer {
    pub fn new(js: String) -> Self {
        Self {
            pos: 0,
            input: js.chars().collect(),
        }
    }

    // keywordが、self.inputの現位置から一致しているかを判断する
    fn contains(&self, keyword: &str) -> bool {
        // self.posから1文字づつ比較して、途中で文字が一致しなくなった場合はfalse
        for i in 0..keyword.len() {
            if keyword.chars().nth(i).expect("failed to access to i-th char")
                != self.input[self.pos + i]
            {
                return false;
            }
        }
        true
    }

    // 予約語かどうかを判断する
    fn check_reserved_word(&self) -> Option<String> {
        for word in RESERVED_WORDS {
            if self.contains(word) {
                return Some(word.to_string());
            }
        }
        None
    }

    // 変数トークンをinputから消費する
    fn consume_identifier(&mut self) -> String {
        let mut result = String::new();

        loop {
            if self.pos >= self.input.len() {
                return result;
            }
            if self.input[self.pos].is_ascii_alphanumeric()
                || self.input[self.pos] == '$'
            {
                result.push(self.input[self.pos]);
                self.pos += 1
            } else {
                return result;
            }
        }
    }

    fn consume_string(&mut self) -> String {
        let mut result = String::new();
        self.pos += 1;

        loop {
            if self.pos >= self.input.len() {
                return result;
            }

            // ダブルクォーとが出てきた時点で、文字列は終了
            if self.input[self.pos] == '"' {
                self.pos += 1;
                return result;
            }
            result.push(self.input[self.pos]);
            self.pos += 1;
        }
    }

    pub fn consume_number(&mut self) -> u64 {
        // 数値型を計算する。
        // "123"の計算の時は、まず1を取り出して、次に2を取り出して、1*10+2とする
        // 12*10 + 3として、次が数値ではないので、break
        let mut result: u64 = 0;

        loop {
            if self.pos >= self.input.len() {
                return result;
            }
            let c = self.input[self.pos];
            match c {
                '0'..='9' => {
                    result = result * 10 + (c.to_digit(10).unwrap() as u64);
                    self.pos += 1;
                }
                _ => break,
            }
        }
        return result;
    }
}

impl Iterator for JsLexer {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.input.len() {
            return None;
        }

        while self.input[self.pos] == ' ' || self.input[self.pos] == '\n' {
            self.pos += 1;

            if self.pos >= self.input.len() {
                return None;
            }
        }

        // 予約後が現れたら、Keywordトークンを返す
        if let Some(reserved_word) = self.check_reserved_word() {
            self.pos += reserved_word.len();
            return Some(Token::Keyword(reserved_word));
        }

        let c = self.input[self.pos];
        let token = match c {
            '+' | '-' | ';' | '=' | '(' | ')' | '{' | '}' | ',' | '.' => {
                let t = Token::Punctuator(c);
                self.pos += 1;
                t
            }
            // 文字の始まりが、変数名として定義できるもののとき
            'a'..='z' | 'A'..='Z' | '_' | '$' => {
                Token::Identifier(self.consume_identifier())
            }
            '0'..='9' => Token::Number(self.consume_number()),
            // 文字列の開始
            '"' => Token::StringLiteral(self.consume_string()),
            _ => unimplemented!("char {:?} is not supported yet", c),
        };
        return Some(token);
    }
}

#[cfg(test)]
mod tests {

    use core::fmt::LowerExp;

    use super::*;

    #[test]
    fn test_empty() {
        let input = "".to_string();
        let mut lexer = JsLexer::new(input).peekable();
        assert!(lexer.peek().is_none());
    }

    #[test]
    fn test_num() {
        let input = "42".to_string();
        let mut lexer = JsLexer::new(input).peekable();
        let expected = [Token::Number(42)].to_vec();
        let mut i = 0;
        while lexer.peek().is_some() {
            assert_eq!(Some(expected[i].clone()), lexer.next());
            i += 1;
        }
        assert!(lexer.peek().is_none());
    }

    #[test]
    fn test_add_numes() {
        let input = "1 + 333".to_string();
        let mut lexer = JsLexer::new(input).peekable();
        let expected =
            [Token::Number(1), Token::Punctuator('+'), Token::Number(333)]
                .to_vec();
        let mut i = 0;
        while lexer.peek().is_some() {
            assert_eq!(Some(expected[i].clone()), lexer.next());
            i += 1;
        }
        assert!(lexer.peek().is_none());
    }

    #[test]
    fn test_assign_variable() {
        let input = "var foo = \"bar\";".to_string();
        let mut lexer = JsLexer::new(input).peekable();
        let expected = [
            Token::Keyword("var".to_string()),
            Token::Identifier("foo".to_string()),
            Token::Punctuator('='),
            Token::StringLiteral("bar".to_string()),
            Token::Punctuator(';'),
        ];
        let mut i = 0;

        while lexer.peek().is_some() {
            assert_eq!(Some(expected[i].clone()), lexer.next());
            i += 1;
        }
    }

    #[test]
    fn test_add_variable_and_num() {
        let input = "var foo = 42; var result = foo + 150;".to_string();
        let mut lexer = JsLexer::new(input).peekable();
        let expected = [
            Token::Keyword("var".to_string()),
            Token::Identifier("foo".to_string()),
            Token::Punctuator('='),
            Token::Number(42),
            Token::Punctuator(';'),
            Token::Keyword("var".to_string()),
            Token::Identifier("result".to_string()),
            Token::Punctuator('='),
            Token::Identifier("foo".to_string()),
            Token::Punctuator('+'),
            Token::Number(150),
            Token::Punctuator(';'),
        ];
        let mut i = 0;

        while lexer.peek().is_some() {
            assert_eq!(Some(expected[i].clone()), lexer.next());
            i += 1;
        }
    }

    #[test]
    fn test_add_local_variable_and_num() {
        let input = r#"
function foo() {
    var a=42; 
    return a;
}
var result = foo() + 1;
"#
        .to_string();
        let mut lexer = JsLexer::new(input).peekable();
        let expected = [
            Token::Keyword("function".to_string()),
            Token::Identifier("foo".to_string()),
            Token::Punctuator('('),
            Token::Punctuator(')'),
            Token::Punctuator('{'),
            Token::Keyword("var".to_string()),
            Token::Identifier("a".to_string()),
            Token::Punctuator('='),
            Token::Number(42),
            Token::Punctuator(';'),
            Token::Keyword("return".to_string()),
            Token::Identifier("a".to_string()),
            Token::Punctuator(';'),
            Token::Punctuator('}'),
            // ここまで関数定義
            Token::Keyword("var".to_string()),
            Token::Identifier("result".to_string()),
            Token::Punctuator('='),
            Token::Identifier("foo".to_string()),
            Token::Punctuator('('),
            Token::Punctuator(')'),
            Token::Punctuator('+'),
            Token::Number(1),
            Token::Punctuator(';'),
        ];
        let mut i = 0;

        while lexer.peek().is_some() {
            assert_eq!(Some(expected[i].clone()), lexer.next());
            i += 1;
        }
    }
}
