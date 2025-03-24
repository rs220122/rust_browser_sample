// ASTを評価して、実行するための基盤

use core::fmt::Display;
use core::fmt::Formatter;
use core::ops::Add;
use core::ops::Sub;

use super::ast::Node;
use super::ast::Program;
use crate::renderer::dom::api::get_element_by_id;
use crate::renderer::dom::node::Node as DomNode;
use crate::renderer::dom::node::NodeKind as DomNodeKind;
use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::borrow::Borrow;
use core::cell::RefCell;

// 関数定義の情報を保持する構造体
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    id: String,
    params: Vec<Option<Rc<Node>>>,
    body: Option<Rc<Node>>,
}

impl Function {
    fn new(
        id: String,
        params: Vec<Option<Rc<Node>>>,
        body: Option<Rc<Node>>,
    ) -> Self {
        Self { id, params, body }
    }
}

#[derive(Debug, Clone)]
pub struct JsRuntime {
    dom_root: Rc<RefCell<DomNode>>,
    functions: Vec<Function>,
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
    HtmlElement {
        object: Rc<RefCell<DomNode>>,
        property: Option<String>,
    },
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
            RuntimeValue::HtmlElement {
                object,
                property: _,
            } => format!("HtmlElement: {:#?}", object),
        };
        write!(f, "{}", s)
    }
}

impl Default for JsRuntime {
    fn default() -> Self {
        Self::new(Rc::new(RefCell::new(DomNode::new(DomNodeKind::Document))))
    }
}

impl JsRuntime {
    pub fn new(dom_root: Rc<RefCell<DomNode>>) -> Self {
        Self {
            dom_root,
            functions: Vec::new(),
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
            Node::FunctionDeclaration { id, params, body } => {
                if let Some(RuntimeValue::StringLiteral(id)) =
                    self.eval(&id, env.clone())
                {
                    let cloned_body = match body {
                        Some(b) => Some(b.clone()),
                        None => None,
                    };
                    // functionsに追加する。
                    self.functions.push(Function::new(
                        id,
                        params.to_vec(),
                        cloned_body,
                    ))
                }
                None
            }
            Node::CallExpression { callee, arguments } => {
                // 新しいスコープをスコープを作成する
                let new_env = Rc::new(RefCell::new(Environment::new(Some(env))));
                let callee_value = match self.eval(callee, new_env.clone()) {
                    Some(value) => value,
                    None => return None,
                };

                let api_result = self.call_browser_api(
                    &callee_value,
                    arguments,
                    new_env.clone(),
                );
                if api_result.0 {
                    // もしブラウザAPIを呼び出していたら、ユーザーが定義した関数を実行しない
                    return api_result.1;
                }

                // すでに定義されている関数を探す
                let function = match self.search_function(callee_value) {
                    Some(func) => func,
                    None => panic!("function {:?} doesn't exist", callee),
                };

                // 関数呼び出し時に渡される引数を新しく作成したスコープのローカル変数としてとして割り当てる
                assert!(arguments.len() == function.params.len());
                for (i, item) in arguments.iter().enumerate() {
                    if let Some(RuntimeValue::StringLiteral(name)) =
                        self.eval(&function.params[i], new_env.clone())
                    {
                        new_env.borrow_mut().add_variable(
                            name,
                            self.eval(item, new_env.clone()),
                        );
                    }
                }
                // 関数の中身を新しいスコープと共にevalメソッドで解釈する
                self.eval(&function.body.clone(), new_env.clone())
            }

            Node::BlockStatement { body } => {
                // 関数呼び出し時にスコープ内のステートメント呼び出す。
                let mut result: Option<RuntimeValue> = None;
                for statement in body {
                    result = self.eval(&statement, env.clone());
                }
                result
            }
            Node::ReturnStatement { argument } => {
                return self.eval(&argument, env.clone());
            }
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

                // leftがDOMツリーのノードを表すHtmlElementならば、DOMツリーを更新する
                if let Some(RuntimeValue::HtmlElement { object, property }) =
                    self.eval(left, env.clone())
                {
                    let right_value = match self.eval(right, env.clone()) {
                        Some(value) => value,
                        None => return None,
                    };

                    if let Some(p) = property {
                        // target.textContent = "foobar";のようにノードのテキストを更新する
                        if p == "textContent" {
                            object.borrow_mut().set_first_child(Some(Rc::new(
                                RefCell::new(DomNode::new(DomNodeKind::Text(
                                    right_value.to_string(),
                                ))),
                            )));
                        }
                    }
                }
                None
            }

            Node::MemberExpression { object, property } => {
                let object_value = match self.eval(object, env.clone()) {
                    Some(value) => value,
                    None => return None,
                };
                let property_value = match self.eval(property, env.clone()) {
                    Some(value) => value,
                    None => return Some(object_value),
                };

                // もしオブジェクトがDOMノードの場合、HtmlELementのpropertyを更新する
                if let RuntimeValue::HtmlElement { object, property } =
                    object_value
                {
                    assert!(property.is_none());
                    // HtmlElementのpropertyにproperty_valueの文字列をセットする。
                    return Some(RuntimeValue::HtmlElement {
                        object,
                        property: Some(property_value.to_string()),
                    });
                }

                // document.getElementByIdは、"document.getElementById"という1つの値として扱う
                // このメソッドのへの呼び出しは、"document.getElementById"という名前への呼び出しになる。
                return Some(
                    object_value
                        + RuntimeValue::StringLiteral(".".to_string())
                        + property_value,
                );
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

    fn search_function(
        &mut self,
        callee_value: RuntimeValue,
    ) -> Option<Function> {
        for func in &self.functions {
            if callee_value == RuntimeValue::StringLiteral(func.id.to_string()) {
                return Some(func.clone());
            }
        }
        None
    }

    /// (bool, Option<RuntimeValue>)のタプルを返す
    /// bool: ブラウザAPIが呼ばれたかどうか。trueなら何かしらのAPIが呼ばれたことを表す
    /// Option<RuntimeValue>: ブラウザAPIの呼び出しによって得られた結果
    /// ブラウザがサポートしているブラウザAPIを呼び出すための関数
    fn call_browser_api(
        &mut self,
        func: &RuntimeValue,
        arguments: &[Option<Rc<Node>>],
        env: Rc<RefCell<Environment>>,
    ) -> (bool, Option<RuntimeValue>) {
        if func
            == &RuntimeValue::StringLiteral(
                "document.getElementById".to_string(),
            )
        {
            let arg = match self.eval(&arguments[0], env.clone()) {
                Some(id) => id,
                None => return (true, None),
            };
            let target = match get_element_by_id(
                Some(self.dom_root.clone()),
                &arg.to_string(),
            ) {
                Some(n) => n,
                None => return (true, None),
            };
            return (
                true,
                Some(RuntimeValue::HtmlElement {
                    object: target,
                    property: None,
                }),
            );
        }
        (false, None)
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
        let runtime = JsRuntime::new(Rc::new(RefCell::new(DomNode::new(
            DomNodeKind::Document,
        ))));
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

    #[test]
    fn test_add_function_and_nums() {
        let (ast, mut runtime) = create_runtime(
            r#"
function foo() {
    return 42;
}
foo() + 1"#
                .to_string(),
        );

        let expected = [None, Some(RuntimeValue::Number(43))];
        for (i, node) in ast.body().iter().enumerate() {
            let result = runtime.eval(&Some(node.clone()), runtime.env.clone());
            assert_eq!(expected[i], result);
        }
    }

    #[test]
    fn test_define_function_with_args() {
        let (ast, mut runtime) = create_runtime(
            r#"
function foo(a, b) {
    return a+b;
}
foo(1, 2) + 3"#
                .to_string(),
        );

        let expected = [None, Some(RuntimeValue::Number(6))];
        for (i, node) in ast.body().iter().enumerate() {
            let result = runtime.eval(&Some(node.clone()), runtime.env.clone());
            assert_eq!(expected[i], result);
        }
    }

    #[test]
    fn test_local_variable() {
        let (ast, mut runtime) = create_runtime(
            r#"
var a = 52;
function foo() {
    var a=1;
    return a;
}
foo() + a"#
                .to_string(),
        );

        let expected = [None, None, Some(RuntimeValue::Number(53))];
        for (i, node) in ast.body().iter().enumerate() {
            let result = runtime.eval(&Some(node.clone()), runtime.env.clone());
            assert_eq!(expected[i], result);
        }

        // env内のテスト
        let env_expected =
            [("a".to_string(), Some(RuntimeValue::Number(52)))].to_vec();
        assert_eq!(runtime.env.borrow_mut().num_variables(), env_expected.len());
        for (name, val) in env_expected {
            assert_eq!(runtime.env.borrow_mut().get_variable(name), val);
        }
    }
}
