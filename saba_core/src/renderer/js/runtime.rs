// ASTを評価して、実行するための基盤

use core::ops::Add;
use core::ops::Sub;

use super::ast::Node;
use super::ast::Program;
use alloc::rc::Rc;
use core::borrow::Borrow;

#[derive(Debug, Clone)]
pub struct JsRuntime {}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeValue {
    Number(u64),
}

impl Add<RuntimeValue> for RuntimeValue {
    type Output = RuntimeValue;

    fn add(self, rhs: RuntimeValue) -> RuntimeValue {
        if let (
            RuntimeValue::Number(left_num),
            RuntimeValue::Number(right_num),
        ) = (&self, &rhs)
        {
            return RuntimeValue::Number(left_num + *right_num);
        }

        RuntimeValue::Number(0)
    }
}

impl Sub<RuntimeValue> for RuntimeValue {
    type Output = RuntimeValue;

    fn sub(self, rhs: RuntimeValue) -> RuntimeValue {
        if let (
            RuntimeValue::Number(left_num),
            RuntimeValue::Number(right_num),
        ) = (&self, &rhs)
        {
            return RuntimeValue::Number(left_num - right_num);
        }
        RuntimeValue::Number(u64::MIN)
    }
}

impl Default for JsRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl JsRuntime {
    pub fn new() -> Self {
        Self {}
    }

    pub fn execute(&mut self, program: &Program) {
        for node in program.body() {
            self.eval(&Some(node.clone()));
        }
    }

    fn eval(&mut self, node: &Option<Rc<Node>>) -> Option<RuntimeValue> {
        let node = match node {
            Some(n) => n,
            None => return None,
        };

        match node.borrow() {
            Node::ExpressionStatement(expr) => return self.eval(expr),
            Node::AdditiveExpression {
                operator,
                left,
                right,
            } => {
                let left_value = match self.eval(left) {
                    Some(value) => value,
                    None => return None,
                };
                let right_value = match self.eval(right) {
                    Some(value) => value,
                    None => return None,
                };

                if operator == &'+' {
                    Some(left_value + right_value)
                } else if operator == &'-' {
                    Some(left_value - right_value)
                } else {
                    None
                }
            }
            Node::AssignmentExpression {
                operator: _,
                left: _,
                right: _,
            } => {
                //
                unimplemented!("not yet")
            }

            Node::MemberExpression {
                object: _,
                property: _,
            } => {
                unimplemented!("not yet")
            }
            Node::NumericLiteral(value) => Some(RuntimeValue::Number(*value)),
            _ => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::{String, ToString};

    use super::*;
    use crate::renderer::js::ast::JsParser;
    use crate::renderer::js::token::JsLexer;

    fn create_runtime(input: String) -> (Program, JsRuntime) {
        let lexer = JsLexer::new(input);
        let mut parser = JsParser::new(lexer);
        let ast = parser.parse_ast();
        let runtime = JsRuntime::new();
        (ast, runtime)
    }

    #[test]
    fn test_num() {
        let (ast, mut runtime) = create_runtime("42".to_string());
        let expected = [Some(RuntimeValue::Number(42))];

        for (i, node) in ast.body().iter().enumerate() {
            let result = runtime.eval(&Some(node.clone()));
            assert_eq!(expected[i], result);
        }
    }

    #[test]
    fn test_add_num() {
        let (ast, mut runtime) = create_runtime("4321 + 12333".to_string());
        let expected = [Some(RuntimeValue::Number(16654))];

        for (i, node) in ast.body().iter().enumerate() {
            let result = runtime.eval(&Some(node.clone()));
            assert_eq!(expected[i], result);
        }
    }

    #[test]
    fn test_sub_nums() {
        let (ast, mut runtime) = create_runtime("11-9".to_string());
        let expected = [Some(RuntimeValue::Number(2))];
        for (i, node) in ast.body().iter().enumerate() {
            let result = runtime.eval(&Some(node.clone()));
            assert_eq!(expected[i], result);
        }
    }
}
