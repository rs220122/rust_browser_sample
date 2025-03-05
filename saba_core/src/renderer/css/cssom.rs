use alloc::{string::String, vec::Vec};

use crate::renderer::css::token::CssTokenizer;
use core::iter::Peekable;

use super::token::CssToken;

#[derive(Debug, Clone)]
pub struct CssParser {
    t: Peekable<CssTokenizer>,
}

impl CssParser {
    pub fn new(t: CssTokenizer) -> Self {
        Self { t: t.peekable() }
    }
}

// セレクター
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selector {
    // タグ名での指定
    TypeSelector(String),
    // クラス名での指定
    ClassSelector(String),
    // IDでの指定
    IdSelector(String),
    /// パース中にエラーが怒った時に使用されるセレクタ
    UnknownSelector,
}

// 宣言ノード
// https://www.w3.org/TR/css-syntax-3/#declaration
#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    // font-colorなどを入れる
    pub property: String,
    // 20pxなどの値を入れる
    pub value: ComponentValue,
}

impl Declaration {
    pub fn new() -> Self {
        Self {
            property: String::new(),
            value: ComponentValue::Ident(String::new()),
        }
    }

    pub fn set_property(&mut self, property: String) {
        self.property = property;
    }

    pub fn set_value(&mut self, value: ComponentValue) {
        self.value = value;
    }
}

// コンポーネント値ノード
// https:///www.w3.org/TR/css-syntax-3/#component-value
pub type ComponentValue = CssToken;

// cssの一つのルール
#[derive(Debug, Clone, PartialEq)]
pub struct QualifiedRule {
    // 公式では、セレクターは1つのルールで複数指定できますが、今回は一つのみとする。（eg. div, #id...)
    pub selector: Selector,
    pub declarations: Vec<Declaration>,
}

impl QualifiedRule {
    pub fn new() -> Self {
        Self {
            selector: Selector::TypeSelector(String::new()),
            declarations: Vec::new(),
        }
    }

    pub fn set_selector(&mut self, selector: Selector) {
        self.selector = selector;
    }

    pub fn set_declarations(&mut self, declarations: Vec<Declaration>) {
        self.declarations = declarations;
    }
}

// CSSOMのルート
#[derive(Debug, Clone, PartialEq)]
pub struct StyleSheet {
    pub rules: Vec<QualifiedRule>,
}

impl StyleSheet {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn set_rules(&mut self, rules: Vec<QualifiedRule>) {
        self.rules = rules;
    }
}
