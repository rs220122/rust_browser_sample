use core::iter::Peekable;

use super::token::JsLexer;
use super::token::Token;
use alloc::{rc::Rc, vec::Vec};

// 字句解析からトークンを受け取って、構文解析して、ASTを作る際のノード
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    ExpressionStatement(Option<Rc<Node>>),
    AdditiveExpression {
        operator: char,
        left: Option<Rc<Node>>,
        right: Option<Rc<Node>>,
    },
    AssignmentExpression {
        operator: char,
        left: Option<Rc<Node>>,
        right: Option<Rc<Node>>,
    },
    MemberExpression {
        object: Option<Rc<Node>>,
        property: Option<Rc<Node>>,
    },
    NumericLiteral(u64),
}

pub struct JsParser {
    t: Peekable<JsLexer>,
}

// ASTを持つ構造体
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    body: Vec<Rc<Node>>,
}

impl JsParser {
    pub fn new(t: JsLexer) -> Self {
        Self { t: t.peekable() }
    }

    pub fn parse_ast(&mut self) -> Program {
        let mut program = Program::new();

        let mut body = Vec::new();
        loop {
            let node = self.source_element();
            match node {
                Some(n) => body.push(n),
                None => {
                    program.set_body(body);
                    return program;
                }
            }
        }
    }

    fn source_element(&mut self) -> Option<Rc<Node>> {
        match self.t.peek() {
            Some(t) => t,
            None => return None,
        };

        self.statement()
    }

    // statementとexpression statementの実装
    // Statement ::= ExpressionStatement
    // ExpressionStatement ::= AssignmentExpression (";")?
    fn statement(&mut self) -> Option<Rc<Node>> {
        let node = Node::new_expression_statement(self.assignment_expression());

        if let Some(Token::Punctuator(c)) = self.t.peek() {
            // ';'を消費する
            if c == &';' {
                assert!(self.t.next().is_some());
            }
        }
        node
    }

    // AssignmentExpression ::= AdditiveExpression
    fn assignment_expression(&mut self) -> Option<Rc<Node>> {
        self.additive_expression()
    }

    // AdditiveExpression ::= LeftHandSizeExpression ( AdditiveOperator AssignmentExpression )*
    fn additive_expression(&mut self) -> Option<Rc<Node>> {
        let left = self.left_hand_size_expression();

        let t = match self.t.peek() {
            Some(token) => token.clone(),
            None => return left,
        };

        match t {
            Token::Punctuator(c) => match c {
                '+' | '-' => {
                    // '_', '-'の時は、その文字列を消費する
                    assert!(self.t.next().is_some());
                    Node::new_additive_expression(
                        c,
                        left,
                        self.assignment_expression(),
                    )
                }
                _ => left,
            },
            _ => left,
        }
    }

    // LeftHandSizeExpression ::= MemberExpression
    fn left_hand_size_expression(&mut self) -> Option<Rc<Node>> {
        self.member_expression()
    }

    // MemberExpression ::= PrimaryExpression
    fn member_expression(&mut self) -> Option<Rc<Node>> {
        self.primary_expression()
    }

    // PrimaryExpression ::= Literal
    // Literal ::= <digit>+
    // <digit> ::= 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9
    fn primary_expression(&mut self) -> Option<Rc<Node>> {
        let t = match self.t.next() {
            Some(token) => token,
            None => return None,
        };

        match t {
            Token::Number(value) => Node::new_numeric_literal(value),
            _ => None,
        }
    }
}

impl Node {
    pub fn new_expression_statement(
        expression: Option<Rc<Self>>,
    ) -> Option<Rc<Self>> {
        Some(Rc::new(Node::ExpressionStatement(expression)))
    }

    pub fn new_additive_expression(
        operator: char,
        left: Option<Rc<Node>>,
        right: Option<Rc<Node>>,
    ) -> Option<Rc<Self>> {
        Some(Rc::new(Node::AdditiveExpression {
            operator,
            left,
            right,
        }))
    }
    pub fn new_assignment_expression(
        operator: char,
        left: Option<Rc<Node>>,
        right: Option<Rc<Node>>,
    ) -> Option<Rc<Self>> {
        Some(Rc::new(Node::AssignmentExpression {
            operator,
            left,
            right,
        }))
    }

    pub fn new_member_expression(
        object: Option<Rc<Node>>,
        property: Option<Rc<Node>>,
    ) -> Option<Rc<Self>> {
        Some(Rc::new(Node::MemberExpression { object, property }))
    }

    pub fn new_numeric_literal(value: u64) -> Option<Rc<Self>> {
        Some(Rc::new(Node::NumericLiteral(value)))
    }
}

impl Default for Program {
    fn default() -> Self {
        Self::new()
    }
}

impl Program {
    pub fn new() -> Self {
        Self { body: Vec::new() }
    }

    pub fn set_body(&mut self, body: Vec<Rc<Node>>) {
        self.body = body;
    }

    pub fn body(&self) -> &Vec<Rc<Node>> {
        &self.body
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::String;
    use alloc::string::ToString;

    fn create_parser(input: String) -> JsParser {
        JsParser::new(JsLexer::new(input))
    }

    #[test]
    fn test_empty() {
        let input = "".to_string();
        let mut parser = create_parser(input);

        let expected = Program::new();
        assert_eq!(expected, parser.parse_ast());
    }

    #[test]
    fn test_num() {
        let input = "53211".to_string();
        let mut parser = create_parser(input);
        let mut expected = Program::new();
        expected.set_body(
            [Rc::new(Node::ExpressionStatement(Some(Rc::new(
                Node::NumericLiteral(53211),
            ))))]
            .to_vec(),
        );

        assert_eq!(expected, parser.parse_ast());
    }

    #[test]
    fn test_add_nums() {
        let input = "216 + 222".to_string();
        let mut parser = create_parser(input);
        let mut expected = Program::new();
        expected.set_body(
            [Rc::new(Node::ExpressionStatement(Some(Rc::new(
                Node::AdditiveExpression {
                    operator: '+',
                    left: Some(Rc::new(Node::NumericLiteral(216))),
                    right: Some(Rc::new(Node::NumericLiteral(222))),
                },
            ))))]
            .to_vec(),
        );

        assert_eq!(expected, parser.parse_ast());
    }

    #[test]
    fn test_minus_nums() {
        let input = "98765 - 1234".to_string();
        let mut parser = create_parser(input);
        let mut expected = Program::new();
        expected.set_body(
            [Rc::new(Node::ExpressionStatement(Some(Rc::new(
                Node::AdditiveExpression {
                    operator: '-',
                    left: Some(Rc::new(Node::NumericLiteral(98765))),
                    right: Some(Rc::new(Node::NumericLiteral(1234))),
                },
            ))))]
            .to_vec(),
        );

        assert_eq!(expected, parser.parse_ast());
    }
}
