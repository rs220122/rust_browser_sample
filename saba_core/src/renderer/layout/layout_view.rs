use super::layout_object::{
    LayoutObject, LayoutObjectKind, LayoutPoint, LayoutSize,
};

use crate::constants::CONTENT_AREA_WIDTH;
use crate::display_item::DisplayItem;
use crate::renderer::dom::api::get_target_element_node;
use crate::renderer::dom::node::Node;
use crate::renderer::layout::layout_object::create_layout_object;
use crate::renderer::{css::cssom::StyleSheet, dom::element::ElementKind};
use alloc::rc::Rc;
use alloc::vec::Vec;
use core::cell::RefCell;

// レイアウトツリーをDOMオブジェクトとcssomから作成する。
fn build_layout_tree(
    node: &Option<Rc<RefCell<Node>>>,
    parent_obj: &Option<Rc<RefCell<LayoutObject>>>,
    cssom: &StyleSheet,
) -> Option<Rc<RefCell<LayoutObject>>> {
    // create_layout_object関数によって、ノードとなるLayoutObjectの作成を行う。
    // CSSによって、display:noneの場合は、ノードは作成されない
    let mut target_node = node.clone();
    let mut layout_object = create_layout_object(node, parent_obj, cssom);

    //もしノードが作成されなかった場合、DOMノードの兄弟ノードを使用して、LayoutObjectの作成をトライする。
    while layout_object.is_none() {
        if let Some(n) = target_node {
            target_node = n.borrow().next_sibling().clone();
            layout_object =
                create_layout_object(&target_node, parent_obj, cssom);
        } else {
            // もし兄弟ノードがない場合、処理するべきDOMツリーは終了
            return layout_object;
        }
    }

    if target_node.is_none() {
        return layout_object;
    }
    let n = target_node.expect("target node should not none");
    let original_first_child = n.borrow().first_child();
    let original_next_sibling = n.borrow().next_sibling();
    let mut first_child =
        build_layout_tree(&original_first_child, &layout_object, cssom);
    let mut next_sibling =
        build_layout_tree(&original_next_sibling, &None, cssom);

    // もし子ノードに"display:none"が指定されていた場合、LayoutObjectは作成されない。
    // その時は、子ノードの兄弟ノードを使用して、LayoutObjectの作成をトライする
    if first_child.is_none() && original_first_child.is_some() {
        let mut original_dom_node = original_first_child
            .expect("first child should exist")
            .borrow()
            .next_sibling();

        loop {
            first_child =
                build_layout_tree(&original_dom_node, &layout_object, cssom);

            if first_child.is_none() && original_dom_node.is_some() {
                original_dom_node = original_dom_node
                    .expect("next sibling should exist")
                    .borrow()
                    .next_sibling();
                continue;
            }
            break;
        }
    }

    // もし兄弟ノードにdisplay:noneが指定されていた場合、LayoutObject
    if next_sibling.is_none() && original_next_sibling.is_some() {
        let mut original_dom_node = original_next_sibling
            .expect("next sibling should exist")
            .borrow()
            .next_sibling();
        loop {
            next_sibling =
                build_layout_tree(&original_dom_node, &layout_object, cssom);

            if next_sibling.is_none() && original_dom_node.is_some() {
                original_dom_node = original_dom_node
                    .expect("next sibling should exist")
                    .borrow()
                    .next_sibling();
                continue;
            }
            break;
        }
    }
    let layout_ref_obj = match layout_object {
        Some(ref obj) => obj,
        None => panic!("render object should exist here"),
    };
    layout_ref_obj.borrow_mut().set_first_child(first_child);
    layout_ref_obj.borrow_mut().set_next_sibling(next_sibling);
    layout_object
}

#[derive(Debug, Clone)]
pub struct LayoutView {
    root: Option<Rc<RefCell<LayoutObject>>>,
}

impl LayoutView {
    pub fn new(root: Rc<RefCell<Node>>, cssom: &StyleSheet) -> Self {
        // レイアウトツリーは描画される要素だけを持つツリーなので、bodyタグ以下の要素をノードとして加える
        let body_root = get_target_element_node(Some(root), ElementKind::Body);

        let mut tree = Self {
            root: build_layout_tree(&body_root, &None, cssom),
        };
        tree.update_layout();
        tree
    }

    fn update_layout(&mut self) {
        Self::calculate_node_size(
            &self.root,
            LayoutSize::new(CONTENT_AREA_WIDTH, 0),
        );
        Self::calculate_node_position(
            &self.root,
            LayoutPoint::new(0, 0),
            LayoutObjectKind::Block,
            None,
            None,
        );
    }

    // レイアウトツリーの各ノードのサイズを再帰的に計算する
    fn calculate_node_size(
        node: &Option<Rc<RefCell<LayoutObject>>>,
        parent_size: LayoutSize,
    ) {
        if let Some(n) = node {
            // ノードがブロック要素の場合、子ノードのレイアウトを計算する前に横幅を決める
            // ブロック要素の時は、親の横幅を引き継ぐ
            if n.borrow().kind() == LayoutObjectKind::Block {
                n.borrow_mut().compute_size(parent_size);
            }

            let first_child = n.borrow().first_child();
            Self::calculate_node_size(&first_child, n.borrow().size());

            let next_sibling = n.borrow().next_sibling();
            Self::calculate_node_size(&next_sibling, parent_size);

            // 子ノードのサイズが決まった後に、サイズを計算する。
            // ブロック要素の時、高さは子ノードの高さに依存する
            n.borrow_mut().compute_size(parent_size);
        }
    }

    fn calculate_node_position(
        node: &Option<Rc<RefCell<LayoutObject>>>,
        parent_point: LayoutPoint,
        previous_sibling_kind: LayoutObjectKind,
        previous_sibling_point: Option<LayoutPoint>,
        previous_sibling_size: Option<LayoutSize>,
    ) {
        if node.is_none() {
            return;
        }
        // 自分のノードの位置を計算する
        let n = node.as_ref().expect("node should exist");
        n.borrow_mut().compute_position(
            parent_point,
            previous_sibling_kind,
            previous_sibling_point,
            previous_sibling_size,
        );

        // ノードの子ノードの位置を計算する
        let first_child = n.borrow().first_child();
        Self::calculate_node_position(
            &first_child,
            n.borrow().point(),
            LayoutObjectKind::Block,
            None,
            None,
        );

        // ノードの兄弟ノードの位置を計算する
        let next_sibling = n.borrow().next_sibling();
        Self::calculate_node_position(
            &next_sibling,
            n.borrow().point(),
            n.borrow().kind(),
            Some(n.borrow().point()),
            Some(n.borrow().size()),
        );
    }

    pub fn root(&self) -> Option<Rc<RefCell<LayoutObject>>> {
        self.root.clone()
    }

    fn paint_node(
        node: &Option<Rc<RefCell<LayoutObject>>>,
        display_items: &mut Vec<DisplayItem>,
    ) {
        match node {
            Some(n) => {
                display_items.extend(n.borrow_mut().paint());
                let first_child = n.borrow().first_child();
                Self::paint_node(&first_child, display_items);
                let next_sibling = n.borrow().next_sibling();
                Self::paint_node(&next_sibling, display_items);
            }
            None => {}
        }
    }

    pub fn paint(&self) -> Vec<DisplayItem> {
        let mut display_items = Vec::new();
        Self::paint_node(&self.root, &mut display_items);
        display_items
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::css::parser::CssParser;
    use crate::renderer::css::token::CssTokenizer;
    use crate::renderer::dom::api::get_style_content;
    use crate::renderer::dom::element::Element;
    use crate::renderer::dom::node::NodeKind;
    use crate::renderer::html::parser::HtmlParser;
    use crate::renderer::html::token::HtmlTokenizer;
    use alloc::string::String;
    use alloc::string::ToString;
    use alloc::vec::Vec;

    fn create_layout_view(html: String) -> LayoutView {
        let t = HtmlTokenizer::new(html);
        let window = HtmlParser::new(t).construct_tree();
        let dom = window.borrow().document();
        let style = get_style_content(dom.clone());
        let css_tokenizer = CssTokenizer::new(style);
        let cssom = CssParser::new(css_tokenizer).parse_stylesheet();
        LayoutView::new(dom, &cssom)
    }

    #[test]
    fn test_empty() {
        let layout_view = create_layout_view("".to_string());
        assert!(layout_view.root.is_none());
    }

    #[test]
    fn test_body() {
        let html = "<html><head></head><body></body></html>".to_string();
        let layout_view = create_layout_view(html);

        let root = layout_view.root();
        assert!(root.is_some());
        assert_eq!(
            LayoutObjectKind::Block,
            root.clone().expect("root should exist").borrow().kind()
        );
        assert_eq!(
            NodeKind::Element(Element::new("body", Vec::new())),
            root.clone().expect("root should exist").borrow().node_kind()
        );
    }

    #[test]
    fn test_text() {
        let html = "<html><head></head><body>text</body></html>".to_string();
        let layout_view = create_layout_view(html);

        let root = layout_view.root();
        assert!(root.is_some());
        assert_eq!(
            NodeKind::Element(Element::new("body", Vec::new())),
            root.clone().expect("root should exist").borrow().node_kind()
        );

        let text = root.expect("root should exist").borrow().first_child();
        assert!(text.is_some());
        assert_eq!(
            LayoutObjectKind::Text,
            text.clone().expect("text should exist").borrow().kind()
        );
    }

    #[test]
    fn test_display_none() {
        let html = "<html><head><style>body{display:none;}</style></head><body>text</body></html>".to_string();
        let layout_view = create_layout_view(html);

        assert!(layout_view.root().is_none());
    }

    #[test]
    fn test_hidden_class() {
        let html = r#"<html>
<head>
<style>
  .hidden {
    display: none;
  }
</style>
</head>
<body>
  <a class="hidden">link1</a>
  <p></p>
  <p class="hidden"><a>link2</a></p>
</body>
</html>"#
            .to_string();

        let layout_view = create_layout_view(html);

        let root = layout_view.root();
        assert!(root.is_some());
        assert_eq!(
            LayoutObjectKind::Block,
            root.clone().expect("root should exist").borrow().kind()
        );
        assert_eq!(
            NodeKind::Element(Element::new("body", Vec::new())),
            root.clone().expect("root should exist").borrow().node_kind()
        );

        let p = root.expect("root should exist").borrow().first_child();
        assert!(p.is_some());
        assert_eq!(
            LayoutObjectKind::Block,
            p.clone().expect("p should exist").borrow().kind()
        );
        assert_eq!(
            NodeKind::Element(Element::new("p", Vec::new())),
            p.clone().expect("p should exist").borrow().node_kind()
        );

        assert!(p
            .clone()
            .expect("p node should exist")
            .borrow()
            .first_child()
            .is_none());
        assert!(p
            .clone()
            .expect("p node should exist")
            .borrow()
            .next_sibling()
            .is_none());
    }
}
