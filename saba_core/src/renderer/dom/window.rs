use crate::renderer::dom::node::Node;
use crate::renderer::dom::node::NodeKind;
use alloc::rc::Rc;
use core::cell::RefCell;

/// DOMツリーのルートを持ち、1つのWebページに対して1つのインスタンスが存在する。
#[derive(Debug, Clone)]
pub struct Window {
    document: Rc<RefCell<Node>>,
}

impl Window {
    pub fn new() -> Self {
        /// DOMツリーのルートノード(ElementKind::Document)を持つように実装を行う。
        let window = Self {
            document: Rc::new(RefCell::new(Node::new(NodeKind::Document))),
        };
        // node.windowに自分の弱い参照を持つようにする。
        window
            .document
            .borrow_mut()
            .set_window(Rc::downgrade(&Rc::new(RefCell::new(window.clone()))));
        window
    }

    pub fn document(&self) -> Rc<RefCell<Node>> {
        self.document.clone()
    }
}
