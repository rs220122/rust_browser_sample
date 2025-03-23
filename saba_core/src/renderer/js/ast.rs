use core::iter::Peekable;

use super::token::JsLexer;
use super::token::Token;
use alloc::string::String;
use alloc::{rc::Rc, vec::Vec};

// 字句解析からトークンを受け取って、構文解析して、ASTを作る際のノード
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    ExpressionStatement(Option<Rc<Node>>),
    VariableDeclaration {
        declarations: Vec<Option<Rc<Node>>>,
    },
    VariableDeclarator {
        id: Option<Rc<Node>>,
        init: Option<Rc<Node>>,
    },
    Identifier(String),
    StringLiteral(String),
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

    // 関数定義で使用するノード
    BlockStatement {
        body: Vec<Option<Rc<Node>>>,
    },
    ReturnStatement {
        argument: Option<Rc<Node>>,
    },
    FunctionDeclaration {
        id: Option<Rc<Node>>,
        params: Vec<Option<Rc<Node>>>,
        body: Option<Rc<Node>>,
    },
    CallExpression {
        callee: Option<Rc<Node>>,
        arguments: Vec<Option<Rc<Node>>>,
    },
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

    // SourceElement ::= Statement | FunctionDeclaration
    fn source_element(&mut self) -> Option<Rc<Node>> {
        let t = match self.t.peek() {
            Some(t) => t,
            None => return None,
        };
        match t {
            Token::Keyword(keyword) => {
                if keyword == "function" {
                    assert!(self.t.next().is_some());
                    self.function_declaration()
                } else {
                    self.statement()
                }
            }
            _ => self.statement(),
        }
    }

    // FunctionDeclaration ::= "function" Identifier "(" (FormalParameterList )? ")" FunctionBody
    fn function_declaration(&mut self) -> Option<Rc<Node>> {
        let id = self.identifier();
        let params = self.parameter_list();
        Node::new_function_declaration(id, params, self.function_body())
    }

    // ParameterList ::= Identifier ( "," Identifier )*
    fn parameter_list(&mut self) -> Vec<Option<Rc<Node>>> {
        let mut params = Vec::new();

        // '('を消費する。
        match self.t.next() {
            Some(t) => match t {
                Token::Punctuator(c) => assert!(c == '('),
                _ => unimplemented!("function should have `(` but got {:?}", t),
            },
            None => unimplemented!("function should have `(` but got None"),
        }

        loop {
            // ')'に到達するまで、paramsに仮引数となる変数を追加する
            match self.t.peek() {
                Some(t) => match t {
                    Token::Punctuator(c) => {
                        if c == &')' {
                            assert!(self.t.next().is_some());
                            return params;
                        }
                        if c == &',' {
                            assert!(self.t.next().is_some());
                        }
                    }
                    _ => {
                        params.push(self.identifier());
                    }
                },
                None => return params,
            }
        }
    }

    // FunctionBody ::= "{" ( SourceElement )? "}"
    fn function_body(&mut self) -> Option<Rc<Node>> {
        // `{`を消費する
        match self.t.next() {
            Some(t) => match t {
                Token::Punctuator(c) => assert!(c == '{'),
                _ => unimplemented!(
                    "function shold have open curly but got {:?}",
                    t
                ),
            },
            None => {
                unimplemented!("function should have open curly but got None")
            }
        }

        let mut body = Vec::new();
        loop {
            if let Some(Token::Punctuator(c)) = self.t.peek() {
                if c == &'}' {
                    assert!(self.t.next().is_some());
                    return Node::new_block_statement(body);
                }
            }
            body.push(self.source_element());
        }
    }

    // statementとexpression statementの実装
    // Statement ::= ExpressionStatement | VariableStatement | RetrunStatement
    // VariableStatement ::= "var" VariableDeclaration
    // ExpressionStatement ::= AssignmentExpression (";")?
    // ReturnStatement ::= "return" AssigmentExpresion (";")?
    fn statement(&mut self) -> Option<Rc<Node>> {
        let t = match self.t.peek() {
            Some(t) => t,
            None => return None,
        };
        let node = match t {
            Token::Keyword(k) => {
                if k == "var" {
                    // "var"を消費
                    assert!(self.t.next().is_some());
                    self.variable_declaration()
                } else if k == "return" {
                    assert!(self.t.next().is_some());
                    Node::new_return_statement(self.assignment_expression())
                } else {
                    None
                }
            }
            _ => Node::new_expression_statement(self.assignment_expression()),
        };

        if let Some(Token::Punctuator(c)) = self.t.peek() {
            // ';'を消費する
            if c == &';' {
                assert!(self.t.next().is_some());
            }
        }
        node
    }

    // VariableDeclaration ::= Identifier ( Initializer )? #
    fn variable_declaration(&mut self) -> Option<Rc<Node>> {
        let ident = self.identifier();

        let declarator =
            Node::new_variable_declarator(ident, self.initializer());

        let declarations = [declarator].to_vec();

        Node::new_variable_declaration(declarations)
    }

    // Identifier ::= <identifier name>
    // <identifier name> ::= (& | _ | a-z | A-Z) (&| a-z | A-Z)*
    fn identifier(&mut self) -> Option<Rc<Node>> {
        let t = match self.t.next() {
            Some(t) => t,
            None => return None,
        };

        match t {
            Token::Identifier(name) => Node::new_identifier(name),
            _ => None,
        }
    }

    // Initializer ::= "=" AssignmentExpression
    fn initializer(&mut self) -> Option<Rc<Node>> {
        let t = match self.t.next() {
            Some(t) => t,
            None => return None,
        };

        if t == Token::Punctuator('=') {
            self.assignment_expression()
        } else {
            None
        }
    }

    // AssignmentExpression ::= AdditiveExpression ( "=" AdditiveExpression )*
    fn assignment_expression(&mut self) -> Option<Rc<Node>> {
        let expr = self.additive_expression();

        let t = match self.t.peek() {
            Some(token) => token,
            None => return expr,
        };

        match t {
            // ("=" AdditiveExpression )* の場合は、こちら
            // 変数の再代入用(example: result = 100)
            Token::Punctuator('=') => {
                // '=' を消費する。
                assert!(self.t.next().is_some());
                Node::new_assignment_expression(
                    '=',
                    expr,
                    self.assignment_expression(),
                )
            }
            _ => expr,
        }
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

    // LeftHandSizeExpression ::= CallExpression | MemberExpression
    fn left_hand_size_expression(&mut self) -> Option<Rc<Node>> {
        let expr = self.member_expression();

        let t = match self.t.peek() {
            Some(token) => token,
            None => return expr,
        };

        match t {
            Token::Punctuator(c) => {
                if c == &'(' {
                    assert!(self.t.next().is_some());
                    return Node::new_call_expression(expr, self.arguments());
                }
                expr
            }
            _ => expr,
        }
    }

    // Arguments ::= "(" ( ArgumentList )? ")"
    // ArgumentList ::= AssignmentExpression ( "," AssignmentExpression )*
    fn arguments(&mut self) -> Vec<Option<Rc<Node>>> {
        let mut arguments = Vec::new();

        loop {
            // ')'に到達するまで、argumentsに引数となる変数を追加する
            match self.t.peek() {
                Some(t) => match t {
                    Token::Punctuator(c) => {
                        if c == &')' {
                            assert!(self.t.next().is_some());
                            return arguments;
                        }
                        if c == &',' {
                            assert!(self.t.next().is_some());
                        }
                    }
                    _ => {
                        arguments.push(self.assignment_expression());
                    }
                },
                None => return arguments,
            }
        }
    }

    // MemberExpression ::= PrimaryExpression ( "." Identifier )?
    fn member_expression(&mut self) -> Option<Rc<Node>> {
        let expr = self.primary_expression();

        let t = match self.t.peek() {
            Some(t) => t,
            None => return expr,
        };

        match t {
            Token::Punctuator(c) => {
                if c == &'.' {
                    assert!(self.t.next().is_some());
                    return Node::new_member_expression(expr, self.identifier());
                }
                expr
            }
            _ => expr,
        }
    }

    // PrimaryExpression ::= Identifier | Literal
    // Literal ::= <digit>+ | <string>
    // <string> ::= " (a-z | A-Z)*"
    // <digit> ::= 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9
    fn primary_expression(&mut self) -> Option<Rc<Node>> {
        let t = match self.t.next() {
            Some(token) => token,
            None => return None,
        };

        match t {
            Token::Number(value) => Node::new_numeric_literal(value),
            Token::StringLiteral(value) => Node::new_string_literal(value),
            Token::Identifier(name) => Node::new_identifier(name),
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

    pub fn new_variable_declarator(
        id: Option<Rc<Self>>,
        init: Option<Rc<Self>>,
    ) -> Option<Rc<Self>> {
        Some(Rc::new(Node::VariableDeclarator { id, init }))
    }

    pub fn new_variable_declaration(
        declarations: Vec<Option<Rc<Self>>>,
    ) -> Option<Rc<Self>> {
        Some(Rc::new(Node::VariableDeclaration { declarations }))
    }

    pub fn new_identifier(name: String) -> Option<Rc<Self>> {
        Some(Rc::new(Node::Identifier(name)))
    }

    pub fn new_string_literal(value: String) -> Option<Rc<Self>> {
        Some(Rc::new(Node::StringLiteral(value)))
    }

    pub fn new_block_statement(body: Vec<Option<Rc<Self>>>) -> Option<Rc<Self>> {
        Some(Rc::new(Node::BlockStatement { body }))
    }

    pub fn new_return_statement(argument: Option<Rc<Self>>) -> Option<Rc<Self>> {
        Some(Rc::new(Node::ReturnStatement { argument }))
    }

    pub fn new_function_declaration(
        id: Option<Rc<Self>>,
        params: Vec<Option<Rc<Self>>>,
        body: Option<Rc<Self>>,
    ) -> Option<Rc<Self>> {
        Some(Rc::new(Node::FunctionDeclaration { id, params, body }))
    }

    pub fn new_call_expression(
        callee: Option<Rc<Self>>,
        arguments: Vec<Option<Rc<Self>>>,
    ) -> Option<Rc<Self>> {
        Some(Rc::new(Node::CallExpression { callee, arguments }))
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

    #[test]
    fn test_assign_variable() {
        let input = "var foo=\"bar\";".to_string();
        let mut parser = create_parser(input);
        let mut expected = Program::new();
        expected.set_body(
            [Rc::new(Node::VariableDeclaration {
                declarations: [Some(Rc::new(Node::VariableDeclarator {
                    id: Some(Rc::new(Node::Identifier("foo".to_string()))),
                    init: Some(Rc::new(Node::StringLiteral("bar".to_string()))),
                }))]
                .to_vec(),
            })]
            .to_vec(),
        );
        assert_eq!(expected, parser.parse_ast());
    }

    #[test]
    fn test_add_variable_and_num() {
        let input = r#"var foo=42; 
var result = foo + 1;"#
            .to_string();
        let mut parser = create_parser(input);
        let mut expected = Program::new();
        expected.set_body(
            [
                Rc::new(Node::VariableDeclaration {
                    declarations: [Some(Rc::new(Node::VariableDeclarator {
                        id: Some(Rc::new(Node::Identifier("foo".to_string()))),
                        init: Some(Rc::new(Node::NumericLiteral(42))),
                    }))]
                    .to_vec(),
                }),
                Rc::new(Node::VariableDeclaration {
                    declarations: [Some(Rc::new(Node::VariableDeclarator {
                        id: Some(Rc::new(Node::Identifier(
                            "result".to_string(),
                        ))),
                        init: Some(Rc::new(Node::AdditiveExpression {
                            operator: '+',
                            left: Some(Rc::new(Node::Identifier(
                                "foo".to_string(),
                            ))),
                            right: Some(Rc::new(Node::NumericLiteral(1))),
                        })),
                    }))]
                    .to_vec(),
                }),
            ]
            .to_vec(),
        );

        assert_eq!(expected, parser.parse_ast());
    }

    #[test]
    fn test_add_variable_and_reassign() {
        // 変数定義(一つの変数)
        // 変数定義(足し算を行ったあとの変数定義)
        // 変数への再代入
        let input = r#"var foo=42; 
var result = foo + 1;
result = 10"#
            .to_string();
        let mut parser = create_parser(input);
        let mut expected = Program::new();
        expected.set_body(
            [
                Rc::new(Node::VariableDeclaration {
                    declarations: [Some(Rc::new(Node::VariableDeclarator {
                        id: Some(Rc::new(Node::Identifier("foo".to_string()))),
                        init: Some(Rc::new(Node::NumericLiteral(42))),
                    }))]
                    .to_vec(),
                }),
                Rc::new(Node::VariableDeclaration {
                    declarations: [Some(Rc::new(Node::VariableDeclarator {
                        id: Some(Rc::new(Node::Identifier(
                            "result".to_string(),
                        ))),
                        init: Some(Rc::new(Node::AdditiveExpression {
                            operator: '+',
                            left: Some(Rc::new(Node::Identifier(
                                "foo".to_string(),
                            ))),
                            right: Some(Rc::new(Node::NumericLiteral(1))),
                        })),
                    }))]
                    .to_vec(),
                }),
                Rc::new(Node::ExpressionStatement(Some(Rc::new(
                    Node::AssignmentExpression {
                        operator: '=',
                        left: Some(Rc::new(Node::Identifier(
                            "result".to_string(),
                        ))),
                        right: Some(Rc::new(Node::NumericLiteral(10))),
                    },
                )))),
            ]
            .to_vec(),
        );

        assert_eq!(expected, parser.parse_ast());
    }

    // 関数定義(引数なし)のテスト
    #[test]
    fn test_define_function_without_arguments() {
        let input = r#"
function foo() {
    return 42;
}"#
        .to_string();
        let mut parser = create_parser(input);
        let mut expected = Program::new();
        let body = [Rc::new(Node::FunctionDeclaration {
            id: Some(Rc::new(Node::Identifier("foo".to_string()))),
            params: Vec::new(),
            body: Some(Rc::new(Node::BlockStatement {
                body: [Some(Rc::new(Node::ReturnStatement {
                    argument: Some(Rc::new(Node::NumericLiteral(42))),
                }))]
                .to_vec(),
            })),
        })]
        .to_vec();

        expected.set_body(body);
        assert_eq!(expected, parser.parse_ast());
    }

    // 関数定義(引数あり)のテスト
    #[test]
    fn test_define_function_with_arguments() {
        let input = r#"
function foo(hoge, fuga) {
    return 42;
}"#
        .to_string();
        let mut parser = create_parser(input);
        let mut expected = Program::new();
        let body = [Rc::new(Node::FunctionDeclaration {
            id: Some(Rc::new(Node::Identifier("foo".to_string()))),
            params: [
                Some(Rc::new(Node::Identifier("hoge".to_string()))),
                Some(Rc::new(Node::Identifier("fuga".to_string()))),
            ]
            .to_vec(),
            body: Some(Rc::new(Node::BlockStatement {
                body: [Some(Rc::new(Node::ReturnStatement {
                    argument: Some(Rc::new(Node::NumericLiteral(42))),
                }))]
                .to_vec(),
            })),
        })]
        .to_vec();

        expected.set_body(body);
        assert_eq!(expected, parser.parse_ast());
    }

    // 関数呼び出しのテスト
    #[test]
    fn test_add_function_add_num() {
        let input = r#"
function foo() {
    return 42;
}
var result = foo() + 555;"#
            .to_string();
        let mut parser = create_parser(input);
        let mut expected = Program::new();
        let body = [
            Rc::new(Node::FunctionDeclaration {
                id: Some(Rc::new(Node::Identifier("foo".to_string()))),
                params: Vec::new(),
                body: Some(Rc::new(Node::BlockStatement {
                    body: [Some(Rc::new(Node::ReturnStatement {
                        argument: Some(Rc::new(Node::NumericLiteral(42))),
                    }))]
                    .to_vec(),
                })),
            }),
            Rc::new(Node::VariableDeclaration {
                declarations: [Some(Rc::new(Node::VariableDeclarator {
                    id: Some(Rc::new(Node::Identifier("result".to_string()))),
                    init: Some(Rc::new(Node::AdditiveExpression {
                        operator: '+',
                        left: Some(Rc::new(Node::CallExpression {
                            callee: Some(Rc::new(Node::Identifier(
                                "foo".to_string(),
                            ))),
                            arguments: Vec::new(),
                        })),
                        right: Some(Rc::new(Node::NumericLiteral(555))),
                    })),
                }))]
                .to_vec(),
            }),
        ]
        .to_vec();

        expected.set_body(body);
        assert_eq!(expected, parser.parse_ast());
    }

    // 関数呼び出し(引数あり)のテスト
    #[test]
    fn test_define_function_and_call_function_with_args() {
        let input = r#"
function foo(hoge, fuga) {
    return 42;
}
foo(100, 400)"#
            .to_string();
        let mut parser = create_parser(input);
        let mut expected = Program::new();
        let body = [
            Rc::new(Node::FunctionDeclaration {
                id: Some(Rc::new(Node::Identifier("foo".to_string()))),
                params: [
                    Some(Rc::new(Node::Identifier("hoge".to_string()))),
                    Some(Rc::new(Node::Identifier("fuga".to_string()))),
                ]
                .to_vec(),
                body: Some(Rc::new(Node::BlockStatement {
                    body: [Some(Rc::new(Node::ReturnStatement {
                        argument: Some(Rc::new(Node::NumericLiteral(42))),
                    }))]
                    .to_vec(),
                })),
            }),
            Rc::new(Node::ExpressionStatement(Some(Rc::new(
                Node::CallExpression {
                    callee: Some(Rc::new(Node::Identifier("foo".to_string()))),
                    arguments: [
                        Some(Rc::new(Node::NumericLiteral(100))),
                        Some(Rc::new(Node::NumericLiteral(400))),
                    ]
                    .to_vec(),
                },
            )))),
        ]
        .to_vec();

        expected.set_body(body);
        assert_eq!(expected, parser.parse_ast());
    }
}
