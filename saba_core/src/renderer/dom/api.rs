use core::cell::RefCell;

use alloc::string::String;
use alloc::vec::Vec;
use alloc::{rc::Rc, string::ToString};

use super::node::Node;
use crate::renderer::dom::element::Element;
use crate::renderer::dom::element::ElementKind;
use crate::renderer::dom::node::NodeKind;

pub fn get_target_element_node(
    node: Option<Rc<RefCell<Node>>>,
    element_kind: ElementKind,
) -> Option<Rc<RefCell<Node>>> {
    match node {
        Some(n) => {
            if n.borrow().kind()
                == NodeKind::Element(Element::new(
                    &element_kind.to_string(),
                    Vec::new(),
                ))
            {
                return Some(n.clone());
            }
            // 深さ優先探索で探す
            let result1 =
                get_target_element_node(n.borrow().first_child(), element_kind);
            let result2 =
                get_target_element_node(n.borrow().next_sibling(), element_kind);

            if result1.is_none() && result2.is_none() {
                return None;
            }
            if result1.is_none() {
                return result2;
            }
            return result1;
        }
        None => None,
    }
}

/// DOMからstyleタグの中身のテキストを取得する
pub fn get_style_content(root: Rc<RefCell<Node>>) -> String {
    let style_node =
        match get_target_element_node(Some(root), ElementKind::Style) {
            Some(node) => node,
            None => return String::new(),
        };

    let text_node = match style_node.borrow().first_child() {
        Some(node) => node,
        None => return String::new(),
    };

    let content = match text_node.borrow().kind() {
        NodeKind::Text(ref s) => s.clone(),
        _ => String::new(),
    };
    content
}
