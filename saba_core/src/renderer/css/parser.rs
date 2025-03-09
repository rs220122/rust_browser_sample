use super::cssom::{ComponentValue, Declaration};
use crate::renderer::css::cssom::{QualifiedRule, Selector, StyleSheet};
use crate::renderer::css::token::CssToken;
use crate::renderer::css::token::CssTokenizer;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::iter::Peekable;

#[derive(Debug, Clone)]
pub struct CssParser {
    t: Peekable<CssTokenizer>,
}

impl CssParser {
    pub fn new(t: CssTokenizer) -> Self {
        Self { t: t.peekable() }
    }

    pub fn parse_stylesheet(&mut self) -> StyleSheet {
        let mut sheet = StyleSheet::new();

        // トークン列からルールのリストを作成し、StyleSheetに設定する。
        sheet.set_rules(self.consume_list_of_rules());
        sheet
    }

    fn consume_list_of_rules(&mut self) -> Vec<QualifiedRule> {
        let mut rules = Vec::new();

        loop {
            // tokenを先読みする。
            let token = match self.t.peek() {
                Some(t) => t,
                None => return rules,
            };

            match token {
                // AtKeyword トークンが出てきた場合、ほかのCSSのインポートする@import, @mediaなどを表す
                CssToken::AtKeyword(_keyword) => {
                    // 今回は、Wから始まるルールはサポートしない
                    let _rule = self.consume_qualified_rule();
                }

                _ => {
                    let rule = self.consume_qualified_rule();
                    if let Some(r) = rule {
                        rules.push(r);
                    } else {
                        return rules;
                    }
                }
            }
        }
    }

    fn consume_qualified_rule(&mut self) -> Option<QualifiedRule> {
        let mut rule = QualifiedRule::new();

        loop {
            let token = match self.t.peek() {
                Some(t) => t,
                None => return None,
            };

            match token {
                // {の後の実際の適用内容を記載するところを解釈する
                CssToken::OpenCurly => {
                    assert_eq!(self.t.next(), Some(CssToken::OpenCurly));
                    rule.set_declarations(self.consume_list_of_declarations());
                    return Some(rule);
                }
                _ => {
                    // セレクターを抽出する
                    rule.set_selector(self.consume_selector());
                }
            }
        }
    }

    fn consume_selector(&mut self) -> Selector {
        let token = match self.t.next() {
            Some(t) => t,
            None => panic!("should have a token but got None"),
        };

        match token {
            // #xxxが指定された場合
            CssToken::HashToken(value) => {
                Selector::IdSelector(value[1..].to_string())
            }
            CssToken::Delim(delim) => {
                if delim == '.' {
                    return Selector::ClassSelector(self.consume_ident());
                }
                panic!("Parse error: {:?} is an expected token.", token);
            }
            CssToken::Ident(ident) => {
                // a:hoverのようなセレクタはタイプセレクタとして扱う
                // コロンが出てきた場合は宣言ブロックの直前までトークンを進める
                // a:hoverは、aとして扱う
                if self.t.peek() == Some(&CssToken::Colon) {
                    while self.t.peek() != Some(&CssToken::OpenCurly) {
                        self.t.next();
                    }
                }
                Selector::TypeSelector(ident.to_string())
            }
            CssToken::AtKeyword(_keyword) => {
                while self.t.peek() != Some(&CssToken::OpenCurly) {
                    self.t.next();
                }
                Selector::UnknownSelector
            }
            _ => {
                self.t.next();
                Selector::UnknownSelector
            }
        }
    }

    fn consume_list_of_declarations(&mut self) -> Vec<Declaration> {
        let mut declarations = Vec::new();

        loop {
            let token = match self.t.peek() {
                Some(t) => t,
                None => return declarations,
            };

            match token {
                CssToken::CloseCurly => {
                    assert_eq!(self.t.next(), Some(CssToken::CloseCurly));
                    return declarations;
                }
                CssToken::SemiColon => {
                    assert_eq!(self.t.next(), Some(CssToken::SemiColon));
                    // 1つの宣言が終了。何もしない。
                }
                CssToken::Ident(ref _ident) => {
                    if let Some(declaration) = self.consume_declaration() {
                        declarations.push(declaration);
                    }
                }
                _ => {
                    self.t.next();
                }
            }
        }
    }

    fn consume_declaration(&mut self) -> Option<Declaration> {
        if self.t.peek().is_none() {
            return None;
        }

        let mut declaration = Declaration::new();
        // 識別子を設定する。 font: xxx; の時のfontの部分
        declaration.set_property(self.consume_ident());
        // もし次のトークンが転んでない場合、パースエラーなのでNoneを返す。
        match self.t.next() {
            Some(token) => match token {
                CssToken::Colon => {}
                _ => return None,
            },
            None => return None,
        }
        declaration.set_value(self.consume_component_value());
        Some(declaration)
    }

    fn consume_ident(&mut self) -> String {
        let token = match self.t.next() {
            Some(t) => t,
            None => panic!("should have a token but got None"),
        };

        match token {
            CssToken::Ident(ref ident) => ident.to_string(),
            _ => {
                panic!("Parse Error: {:?} is an unexpected token", token);
            }
        }
    }

    fn consume_component_value(&mut self) -> ComponentValue {
        self.t.next().expect("should have a consume_component_value")
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use alloc::vec;

    fn create_stylesheet(style: String) -> StyleSheet {
        CssParser::new(CssTokenizer::new(style)).parse_stylesheet()
    }

    #[test]
    fn test_empty() {
        let cssom = create_stylesheet("".to_string());
        assert_eq!(cssom.rules.len(), 0)
    }

    #[test]
    fn test_one_rule() {
        let cssom = create_stylesheet("p {color: red;}".to_string());
        let mut rule = QualifiedRule::new();
        rule.set_selector(Selector::TypeSelector("p".to_string()));
        let mut declaration = Declaration::new();
        declaration.set_property("color".to_string());
        declaration.set_value(ComponentValue::Ident("red".to_string()));
        rule.set_declarations(vec![declaration]);

        let expected = [rule];
        assert_eq!(cssom.rules.len(), expected.len());

        for (i, rule) in cssom.rules.iter().enumerate() {
            assert_eq!(&expected[i], rule);
        }
    }

    #[test]
    fn test_id_selector() {
        let cssom = create_stylesheet("#id {color: blue;}".to_string());

        let mut rule = QualifiedRule::new();
        rule.set_selector(Selector::IdSelector("id".to_string()));
        let mut declaration = Declaration::new();
        declaration.set_property("color".to_string());
        declaration.set_value(ComponentValue::Ident("blue".to_string()));
        rule.set_declarations(vec![declaration]);

        let expected = [rule];
        assert_eq!(cssom.rules.len(), expected.len());

        for (i, rule) in cssom.rules.iter().enumerate() {
            assert_eq!(&expected[i], rule);
        }
    }

    #[test]
    fn test_class_selector() {
        let cssom = create_stylesheet(".test_class {color: blue;}".to_string());

        let mut rule = QualifiedRule::new();
        rule.set_selector(Selector::ClassSelector("test_class".to_string()));
        let mut declaration = Declaration::new();
        declaration.set_property("color".to_string());
        declaration.set_value(ComponentValue::Ident("blue".to_string()));
        rule.set_declarations(vec![declaration]);

        let expected = [rule];
        assert_eq!(cssom.rules.len(), expected.len());

        for (i, rule) in cssom.rules.iter().enumerate() {
            assert_eq!(&expected[i], rule);
        }
    }

    #[test]
    fn test_multiple_rules() {
        let cssom = create_stylesheet(
            ".test_class {color: blue;} h1 {font-size: 40; color: white;}"
                .to_string(),
        );

        let mut rule1 = QualifiedRule::new();
        rule1.set_selector(Selector::ClassSelector("test_class".to_string()));
        let mut declaration = Declaration::new();
        declaration.set_property("color".to_string());
        declaration.set_value(ComponentValue::Ident("blue".to_string()));
        rule1.set_declarations(vec![declaration]);

        let mut rule2 = QualifiedRule::new();
        rule2.set_selector(Selector::TypeSelector("h1".to_string()));
        let mut d1 = Declaration::new();
        let mut d2 = Declaration::new();
        d1.set_property("font-size".to_string());
        d1.set_value(ComponentValue::Number(40.0));
        d2.set_property("color".to_string());
        d2.set_value(ComponentValue::Ident("white".to_string()));
        rule2.set_declarations(vec![d1, d2]);

        let expected = [rule1, rule2];
        assert_eq!(cssom.rules.len(), expected.len());

        for (index, rule) in cssom.rules.iter().enumerate() {
            assert_eq!(rule, &expected[index]);
        }
    }
}
