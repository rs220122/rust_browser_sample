use core::cell::RefCell;

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
