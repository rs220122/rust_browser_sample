// ASTを評価して、実行するための基盤

use core::fmt::Display;
use core::fmt::Formatter;
use core::ops::Add;
use core::ops::Sub;

use super::ast::Node;
use super::ast::Program;
use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::borrow::Borrow;
use core::cell::RefCell;

#[derive(Debug, Clone)]
pub struct JsRuntime {
    env: Rc<RefCell<Environment>>,
}

// 変数名とその変数の値を管理する辞書
type VariableMap = Vec<(String, Option<RuntimeValue>)>;

// スコープ内の変数を管理する構造体
// outerは、スコープよりも外側の変数にアクセスするために使用する。
#[derive(Debug, Clone)]
pub struct Environment {
    variables: VariableMap,
    outer: Option<Rc<RefCell<Environment>>>,
}

impl Environment {
    fn new(outer: Option<Rc<RefCell<Environment>>>) -> Self {
        Self {
            variables: VariableMap::new(),
            outer,
        }
    }

    pub fn get_variable(&self, name: String) -> Option<RuntimeValue> {
        for variable in &self.variables {
            if variable.0 == name {
                return variable.1.clone();
            }
        }

        // スコープの外側を再帰的に探す
        if let Some(env) = &self.outer {
            env.borrow_mut().get_variable(name)
        } else {
            None
        }
    }

    // スコープ内に変数を追加する
    pub fn add_variable(&mut self, name: String, value: Option<RuntimeValue>) {
        self.variables.push((name, value));
    }

    // スコープ内の変数の値を更新する
    fn update_variable(&mut self, name: String, value: Option<RuntimeValue>) {
        for i in 0..self.variables.len() {
            if self.variables[i].0 == name {
                self.variables.remove(i);
                self.variables.push((name, value));
                return;
            }
        }
    }

    pub fn num_variables(&self) -> usize {
        self.variables.len()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeValue {
    Number(u64),
    StringLiteral(String),
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
        // どちらかが文字列 or どちらも文字列の場合は、文字列の結合として扱う
        RuntimeValue::StringLiteral(self.to_string() + &rhs.to_string())
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

        // 整数以外の引き算のときは、全て無効な値として、u64::MINとする。
        RuntimeValue::Number(u64::MIN)
    }
}

impl Display for RuntimeValue {
    // RuntimeValueで、to_stringを使えるように
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        let s = match self {
            RuntimeValue::Number(value) => format!("{}", value),
            RuntimeValue::StringLiteral(value) => value.to_string(),
        };
        write!(f, "{}", s)
    }
}

impl Default for JsRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl JsRuntime {
    pub fn new() -> Self {
        Self {
            env: Rc::new(RefCell::new(Environment::new(None))),
        }
    }

    pub fn execute(&mut self, program: &Program) {
        for node in program.body() {
            self.eval(&Some(node.clone()), self.env.clone());
        }
    }

    fn eval(
        &mut self,
        node: &Option<Rc<Node>>,
        env: Rc<RefCell<Environment>>,
    ) -> Option<RuntimeValue> {
        let node = match node {
            Some(n) => n,
            None => return None,
        };

        match node.borrow() {
            Node::ExpressionStatement(expr) => {
                return self.eval(expr, env.clone())
            }
            Node::AdditiveExpression {
                operator,
                left,
                right,
            } => {
                let left_value = match self.eval(left, env.clone()) {
                    Some(value) => value,
                    None => return None,
                };
                let right_value = match self.eval(right, env.clone()) {
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
                operator,
                left,
                right,
            } => {
                if operator != &'=' {
                    return None;
                }
                // 変数の再割り当て
                if let Some(node) = left {
                    if let Node::Identifier(id) = node.borrow() {
                        let new_value = self.eval(right, env.clone());
                        env.borrow_mut()
                            .update_variable(id.to_string(), new_value);
                    }
                }
                None
            }

            Node::MemberExpression {
                object: _,
                property: _,
            } => {
                unimplemented!("not yet")
            }
            Node::NumericLiteral(value) => Some(RuntimeValue::Number(*value)),
            Node::VariableDeclaration { declarations } => {
                for dec in declarations {
                    self.eval(dec, env.clone());
                }
                None
            }
            Node::VariableDeclarator { id, init } => {
                // var a = 10;のような変数定義の時にここに入り、aが、Identifierで、10がRuntimeValueとなる。
                if let Some(node) = id {
                    if let Node::Identifier(name) = node.borrow() {
                        let init = self.eval(init, env.clone());
                        env.borrow_mut().add_variable(name.to_string(), init);
                    }
                }
                None
            }
            Node::Identifier(name) => {
                match env.borrow_mut().get_variable(name.to_string()) {
                    Some(v) => Some(v),
                    // 変数名が初めて使用される場合は、まだ値が保存されていないので、文字列として扱う
                    // example: var a= 42; のような時に、aが変数としてない時は、
                    // aは、StringLiteralとなる
                    None => Some(RuntimeValue::StringLiteral(name.to_string())),
                }
            }
            Node::StringLiteral(value) => {
                Some(RuntimeValue::StringLiteral(value.to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use core::ops::Deref;

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
            let result = runtime.eval(&Some(node.clone()), runtime.env.clone());
            assert_eq!(expected[i], result);
        }
    }

    #[test]
    fn test_add_num() {
        let (ast, mut runtime) = create_runtime("4321 + 12333".to_string());
        let expected = [Some(RuntimeValue::Number(16654))];

        for (i, node) in ast.body().iter().enumerate() {
            let result = runtime.eval(&Some(node.clone()), runtime.env.clone());
            assert_eq!(expected[i], result);
        }
    }

    #[test]
    fn test_sub_nums() {
        let (ast, mut runtime) = create_runtime("11-9".to_string());
        let expected = [Some(RuntimeValue::Number(2))];
        for (i, node) in ast.body().iter().enumerate() {
            let result = runtime.eval(&Some(node.clone()), runtime.env.clone());
            assert_eq!(expected[i], result);
        }
    }

    #[test]
    fn test_assign_variable() {
        let (ast, mut runtime) = create_runtime("var foo = 42;".to_string());
        let expected = [None];

        for (i, node) in ast.body().iter().enumerate() {
            let result = runtime.eval(&Some(node.clone()), runtime.env.clone());
            assert_eq!(expected[i], result);
        }

        // env内のテスト
        let env_expected =
            [("foo".to_string(), Some(RuntimeValue::Number(42)))].to_vec();

        assert_eq!(runtime.env.borrow_mut().num_variables(), env_expected.len());
        for (name, val) in env_expected {
            assert_eq!(runtime.env.borrow_mut().get_variable(name), val);
        }
    }

    #[test]
    fn test_add_variable_and_num() {
        let (ast, mut runtime) = create_runtime("var foo=42;foo+1;".to_string());
        let expected = [None, Some(RuntimeValue::Number(43))];

        for (i, node) in ast.body().iter().enumerate() {
            let result = runtime.eval(&Some(node.clone()), runtime.env.clone());
            assert_eq!(expected[i], result);
        }

        // env内のテスト
        let env_expected =
            [("foo".to_string(), Some(RuntimeValue::Number(42)))].to_vec();

        assert_eq!(runtime.env.borrow_mut().num_variables(), env_expected.len());
        for (name, val) in env_expected {
            assert_eq!(runtime.env.borrow_mut().get_variable(name), val);
        }
    }

    #[test]
    fn test_reassign_variable() {
        let (ast, mut runtime) =
            create_runtime("var foo=42; foo=150;foo".to_string());
        let expected = [None, None, Some(RuntimeValue::Number(150))];

        for (i, node) in ast.body().iter().enumerate() {
            let result = runtime.eval(&Some(node.clone()), runtime.env.clone());
            assert_eq!(expected[i], result);
        }

        // env内のテスト
        let env_expected =
            [("foo".to_string(), Some(RuntimeValue::Number(150)))].to_vec();

        assert_eq!(runtime.env.borrow_mut().num_variables(), env_expected.len());
        for (name, val) in env_expected {
            assert_eq!(runtime.env.borrow_mut().get_variable(name), val);
        }
    }

    #[test]
    fn test_reaasing_and_add_string() {
        let (ast, mut runtime) = create_runtime(
            r#"
var foo = 150;
foo = 523;
var a = 100;
a = 100 + "aaa";
foo = "abc" + 532;
var b = 150 - "aaa";
"#
            .to_string(),
        );

        let expected = [None, None, None, None, None, None];
        for (i, node) in ast.body().iter().enumerate() {
            let result = runtime.eval(&Some(node.clone()), runtime.env.clone());
            assert_eq!(expected[i], result);
        }

        // env内のテスト
        let env_expected = [
            (
                "foo".to_string(),
                Some(RuntimeValue::StringLiteral("abc532".to_string())),
            ),
            (
                "a".to_string(),
                Some(RuntimeValue::StringLiteral("100aaa".to_string())),
            ),
            ("b".to_string(), Some(RuntimeValue::Number(u64::MIN))),
        ]
        .to_vec();

        assert_eq!(runtime.env.borrow_mut().num_variables(), env_expected.len());
        for (name, val) in env_expected {
            assert_eq!(runtime.env.borrow_mut().get_variable(name), val);
        }
    }
}
