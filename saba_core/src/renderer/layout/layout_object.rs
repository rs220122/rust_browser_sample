use super::computed_style::{Color, ComputedStyle, FontSize};
use crate::constants::{
    CHAR_HEIGHT_WITH_PADDING, CHAR_WIDTH, CONTENT_AREA_WIDTH, WINDOW_PADDING,
    WINDOW_WIDTH,
};
use crate::display_item::DisplayItem;

use crate::renderer::css::cssom::{ComponentValue, Declaration};
use crate::renderer::dom::node::NodeKind;
use crate::renderer::layout::computed_style::DisplayType;
use crate::renderer::{
    css::cssom::{Selector, StyleSheet},
    dom::node::Node,
};
use alloc::rc::{Rc, Weak};
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;
use core::{cell::RefCell, i64};

/// 1行の文字数を超えた場合、改行すべき適切な位置を見つける
/// 要素を超えた場合は、単語の区切りで改行する
fn find_index_for_line_break(line: String, max_index: usize) -> usize {
    for i in (0..max_index).rev() {
        if line.chars().collect::<Vec<char>>()[i] == ' ' {
            return i;
        }
    }
    max_index
}

/// https://drafts.csswg.org/css-text/#word-break-property
/// char_width: 1文字の幅
fn split_text(line: String, char_width: i64) -> Vec<String> {
    let mut result: Vec<String> = vec![];
    if line.len() as i64 * char_width > (WINDOW_WIDTH + WINDOW_PADDING) {
        // WINDOW_WIDTH + WINDOW_PADDINGは、一行に収まる最大のエリア
        // (WINDOW_WIDTH + WINDOW_PADDING)が100の場合、
        // char_widthが5の場合、20文字が一行に収まる
        // その場合20-0文字の中で、空白のものを探し、そこで改行する
        let s = line.split_at(find_index_for_line_break(
            line.clone(),
            ((WINDOW_WIDTH + WINDOW_PADDING) / char_width) as usize,
        ));
        result.push(s.0.to_string());
        result.extend(split_text(s.1.trim().to_string(), char_width))
    } else {
        result.push(line);
    }
    result
}

// layout_objectを作成する。
// computed_styleを正しくもつ為に、ここで、宣言値の決定と指定値の決定を行う
pub fn create_layout_object(
    node: &Option<Rc<RefCell<Node>>>,
    parent_obj: &Option<Rc<RefCell<LayoutObject>>>,
    cssom: &StyleSheet,
) -> Option<Rc<RefCell<LayoutObject>>> {
    if let Some(n) = node {
        // create layout object
        let layout_object =
            Rc::new(RefCell::new(LayoutObject::new(n.clone(), parent_obj)));

        // CSSのルールをセレクタで選択されたノードに適用する
        for rule in &cssom.rules {
            if layout_object.borrow().is_node_selected(&rule.selector) {
                // 宣言値の設定を行う
                layout_object
                    .borrow_mut()
                    .cascading_style(rule.declarations.clone());
            }
        }

        // CSSでスタイルが指定されていない場合、デフォルトの値または親ノードから継承した値を使用する
        let parent_style = if let Some(parent) = parent_obj {
            Some(parent.borrow().style())
        } else {
            None
        };
        // 指定値の決定を行う
        layout_object.borrow_mut().defaulting_style(n, parent_style);

        // displayプロパティがnoneの場合、ノードを作成しない
        if layout_object.borrow().style().display() == DisplayType::DisplayNone {
            return None;
        }

        // displayプロパティの最終的な値を使用してノードの種類を決定する
        layout_object.borrow_mut().update_kind();
        return Some(layout_object);
    }
    None
}

#[derive(Debug, Clone)]
pub struct LayoutObject {
    kind: LayoutObjectKind,
    node: Rc<RefCell<Node>>,
    first_child: Option<Rc<RefCell<LayoutObject>>>,
    next_sibling: Option<Rc<RefCell<LayoutObject>>>,
    parent: Weak<RefCell<LayoutObject>>,
    style: ComputedStyle,
    point: LayoutPoint,
    size: LayoutSize,
}

impl LayoutObject {
    pub fn new(
        node: Rc<RefCell<Node>>,
        parent_obj: &Option<Rc<RefCell<LayoutObject>>>,
    ) -> Self {
        let parent = match parent_obj {
            Some(p) => Rc::downgrade(p),
            None => Weak::new(),
        };

        Self {
            kind: LayoutObjectKind::Block,
            node: node.clone(),
            first_child: None,
            next_sibling: None,
            parent,
            style: ComputedStyle::new(),
            point: LayoutPoint::new(0, 0),
            size: LayoutSize::new(0, 0),
        }
    }

    pub fn is_node_selected(&self, selector: &Selector) -> bool {
        match self.node_kind() {
            NodeKind::Element(elem) => match selector {
                //　attributesのIDを比較して、一致している場合はtrue
                Selector::IdSelector(ident) => {
                    for attr in elem.attributes() {
                        if attr.name() == "id" && attr.value() == *ident {
                            return true;
                        }
                    }
                    false
                }
                // attirbutesのclassを比較して、一致している場合は、true
                Selector::ClassSelector(class_name) => {
                    for attr in elem.attributes() {
                        if attr.name() == "class" && attr.value() == *class_name
                        {
                            return true;
                        }
                    }
                    false
                }
                // このnodeのnodekindが一致している場合は、true
                Selector::TypeSelector(tag) => {
                    if elem.kind().to_string() == *tag {
                        return true;
                    }
                    false
                }
                Selector::UnknownSelector => false,
            },
            _ => false,
        }
    }

    pub fn cascading_style(&mut self, declarations: Vec<Declaration>) {
        for declaration in declarations {
            match declaration.property.as_str() {
                "background-color" => {
                    if let ComponentValue::Ident(value) = &declaration.value {
                        let color = match Color::from_name(&value) {
                            Ok(color) => color,
                            Err(_) => Color::white(),
                        };
                        self.style.set_background_color(color);
                        continue;
                    }
                    if let ComponentValue::HashToken(value) = &declaration.value
                    {
                        let color = match Color::from_code(&value) {
                            Ok(color) => color,
                            Err(_) => Color::white(),
                        };
                        self.style.set_background_color(color);
                        continue;
                    }
                }
                "color" => {
                    if let ComponentValue::Ident(value) = &declaration.value {
                        let color = match Color::from_name(&value) {
                            Ok(color) => color,
                            Err(_) => Color::black(),
                        };
                        self.style.set_color(color);
                        continue;
                    }
                    if let ComponentValue::HashToken(value) = &declaration.value
                    {
                        let color = match Color::from_code(&value) {
                            Ok(color) => color,
                            Err(_) => Color::black(),
                        };
                        self.style.set_color(color);
                        continue;
                    }
                }
                "display" => {
                    if let ComponentValue::Ident(value) = &declaration.value {
                        let display_type = match DisplayType::from_str(&value) {
                            Ok(display_type) => display_type,
                            Err(_) => DisplayType::DisplayNone,
                        };
                        self.style.set_display(display_type);
                        continue;
                    }
                }
                _ => {}
            }
        }
    }

    pub fn defaulting_style(
        &mut self,
        node: &Rc<RefCell<Node>>,
        parent_style: Option<ComputedStyle>,
    ) {
        self.style.defaulting(node, parent_style);
    }

    pub fn update_kind(&mut self) {
        self.kind = match self.node_kind() {
            NodeKind::Document => {
                panic!("should not create a layout object for a Document node")
            }
            NodeKind::Element(_) => match self.style().display() {
                DisplayType::DisplayNone => {
                    panic!("should not create a layout object for display:none")
                }
                DisplayType::Block => LayoutObjectKind::Block,
                DisplayType::Inline => LayoutObjectKind::Inline,
            },
            NodeKind::Text(_) => LayoutObjectKind::Text,
        };
    }

    pub fn compute_size(&mut self, parent_size: LayoutSize) {
        let mut size = LayoutSize::new(0, 0);

        match self.kind() {
            LayoutObjectKind::Block => {
                size.set_width(parent_size.width());

                // 全ての子ノードの高さを足し合わせた結果が高さになる。
                // ただし、インライン要素が横に並んでいる場合は、注意が必要
                let mut height = 0;
                let mut child = self.first_child();
                let mut previous_child_kind = LayoutObjectKind::Block;
                while child.is_some() {
                    let c = match child {
                        Some(c) => c,
                        None => panic!("first child should exist"),
                    };
                    if previous_child_kind == LayoutObjectKind::Block
                        || c.borrow().kind() == LayoutObjectKind::Block
                    {
                        height += c.borrow().size().height();
                    }

                    previous_child_kind = c.borrow().kind();
                    child = c.borrow().next_sibling();
                }
                size.set_height(height);
            }

            LayoutObjectKind::Inline => {
                // すべての子ノードの高さと横幅を足し合わせて結果が現在のノードの高さと横幅となる。
                let mut width = 0;
                let mut height = 0;
                let mut child = self.first_child();
                while child.is_some() {
                    let c = child.expect("child should exist");
                    width += c.borrow().size().width();
                    height += c.borrow().size().height();
                    child = c.borrow().next_sibling();
                }
                size.set_height(height);
                size.set_width(width);
            }

            LayoutObjectKind::Text => {
                if let NodeKind::Text(t) = self.node_kind() {
                    let ratio = match self.style.font_size() {
                        FontSize::Medium => 1,
                        FontSize::XLarge => 2,
                        FontSize::XXLarge => 3,
                    };
                    let width = CHAR_WIDTH * ratio * t.len() as i64;
                    if width > CONTENT_AREA_WIDTH {
                        // テキストが複数行の時
                        size.set_width(CONTENT_AREA_WIDTH);
                        let line_num =
                            if width.wrapping_rem(CONTENT_AREA_WIDTH) == 0 {
                                width.wrapping_div(CONTENT_AREA_WIDTH)
                            } else {
                                width.wrapping_div(CONTENT_AREA_WIDTH) + 1
                            };
                        size.set_height(
                            line_num * ratio * CHAR_HEIGHT_WITH_PADDING,
                        );
                    } else {
                        // テキストが一行に収まる時
                        size.set_width(width);
                        size.set_height(ratio * CHAR_HEIGHT_WITH_PADDING);
                    }
                }
            }
        }
        self.size = size;
    }

    pub fn compute_position(
        &mut self,
        parent_point: LayoutPoint,
        previous_sibling_kind: LayoutObjectKind,
        previous_sibling_point: Option<LayoutPoint>,
        previous_sibling_size: Option<LayoutSize>,
    ) {
        let mut point = LayoutPoint::new(0, 0);

        match (self.kind(), previous_sibling_kind) {
            // 兄弟要素がブロック要素の場合は、Y座標を足し合わせる
            (LayoutObjectKind::Block, _) | (_, LayoutObjectKind::Block) => {
                if let (Some(size), Some(pos)) =
                    (previous_sibling_size, previous_sibling_point)
                {
                    point.set_y(pos.y() + size.height());
                } else {
                    // 兄弟要素が存在しない場合は、親要素のY座標を基準にする
                    point.set_y(parent_point.y());
                }
                point.set_x(parent_point.x());
            }
            //
            (LayoutObjectKind::Inline, LayoutObjectKind::Inline) => {
                if let (Some(size), Some(pos)) =
                    (previous_sibling_size, previous_sibling_point)
                {
                    point.set_x(pos.x() + size.width());
                    point.set_y(pos.y());
                } else {
                    point.set_x(parent_point.x());
                    point.set_y(parent_point.y());
                }
            }
            _ => {
                point.set_x(parent_point.x());
                point.set_y(parent_point.y());
            }
        }

        self.point = point;
    }

    pub fn paint(&mut self) -> Vec<DisplayItem> {
        if self.style.display() == DisplayType::DisplayNone {
            return vec![];
        }

        match self.kind {
            LayoutObjectKind::Block => {
                if let NodeKind::Element(_e) = self.node_kind() {
                    return vec![DisplayItem::Rect {
                        style: self.style(),
                        layout_point: self.point(),
                        layout_size: self.size(),
                    }];
                }
            }
            LayoutObjectKind::Inline => {
                // 本書の無ライザでは、描画するインライン要素はない
                // <img>タグなどをサポートした場合はこのアーム中で処理する
            }
            LayoutObjectKind::Text => {
                if let NodeKind::Text(t) = self.node_kind() {
                    let mut v = vec![];
                    let ratio = match self.style.font_size() {
                        FontSize::Medium => 1,
                        FontSize::XLarge => 2,
                        FontSize::XXLarge => 3,
                    };

                    // テキスト内に含まれる改行を削除し、単語ごとに分割する
                    let plain_text = t
                        .replace("\n", "")
                        .split(' ')
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                        .join(" ");
                    // 描画領域に収まるようにテキストを分割する
                    let lines = split_text(plain_text, CHAR_WIDTH * ratio);
                    for (i, line) in lines.into_iter().enumerate() {
                        let item = DisplayItem::Text {
                            text: line,
                            style: self.style(),
                            layout_point: LayoutPoint::new(
                                self.point().x(),
                                self.point().y()
                                    + CHAR_HEIGHT_WITH_PADDING * i as i64,
                            ),
                        };
                        v.push(item);
                    }
                    return v;
                }
            }
        }
        vec![]
    }

    pub fn kind(&self) -> LayoutObjectKind {
        self.kind
    }

    pub fn node_kind(&self) -> NodeKind {
        self.node.borrow().kind().clone()
    }

    pub fn set_first_child(
        &mut self,
        first_child: Option<Rc<RefCell<LayoutObject>>>,
    ) {
        self.first_child = first_child;
    }
    pub fn first_child(&self) -> Option<Rc<RefCell<LayoutObject>>> {
        self.first_child.as_ref().cloned()
    }

    pub fn set_next_sibling(
        &mut self,
        next_sibling: Option<Rc<RefCell<LayoutObject>>>,
    ) {
        self.next_sibling = next_sibling
    }

    pub fn next_sibling(&self) -> Option<Rc<RefCell<LayoutObject>>> {
        self.next_sibling.as_ref().cloned()
    }

    pub fn parent(&self) -> Weak<RefCell<Self>> {
        self.parent.clone()
    }

    pub fn style(&self) -> ComputedStyle {
        self.style.clone()
    }

    pub fn point(&self) -> LayoutPoint {
        self.point
    }

    pub fn size(&self) -> LayoutSize {
        self.size
    }
}

impl PartialEq for LayoutObject {
    fn eq(&self, other: &Self) -> bool {
        // LayoutObjectKindが等しい かつ
        // LayoutPointが等しい かつ
        // LayoutSizeが等しい かつ
        self.kind == other.kind
            && self.point == other.point
            && self.size == other.size
    }
}

// コンテンツの表示方法の種類を定義。
// inline要素とblock要素、
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LayoutObjectKind {
    Block,
    Inline,
    Text,
}

// LayoutObjectの位置を表す構造体。各要素の描画される位置を計算する
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutPoint {
    x: i64,
    y: i64,
}

impl LayoutPoint {
    pub fn new(x: i64, y: i64) -> Self {
        Self { x, y }
    }

    pub fn x(&self) -> i64 {
        self.x
    }

    pub fn y(&self) -> i64 {
        self.y
    }

    pub fn set_x(&mut self, x: i64) {
        self.x = x;
    }
    pub fn set_y(&mut self, y: i64) {
        self.y = y;
    }
}

// LayoutObjectの描画するサイズを表す構造体
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutSize {
    width: i64,
    height: i64,
}

impl LayoutSize {
    pub fn new(width: i64, height: i64) -> Self {
        Self { width, height }
    }

    pub fn width(&self) -> i64 {
        self.width
    }
    pub fn height(&self) -> i64 {
        self.height
    }

    pub fn set_height(&mut self, height: i64) {
        self.height = height;
    }

    pub fn set_width(&mut self, width: i64) {
        self.width = width;
    }
}
