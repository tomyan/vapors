//! The box tree: what Taffy lays out, derived from the document on each
//! update. An element with `display: none` has no box; each run of
//! consecutive text nodes becomes an anonymous text box whose text is
//! collapsed as `white-space: normal` collapses it, and a run that
//! collapses to nothing has no box (the empty anchors `{if}` and `{for}`
//! leave, and the whitespace between blocks).

use std::collections::HashMap;

use taffy::{Cache, Display, Layout, Style};
use vapors_dom::NodeId;

use crate::arena::Arena;
use crate::node::Kind;
use crate::style::{TextStyle, Used};

/// What a box is.
pub(crate) enum Content {
    /// An element's box; its children are boxes too.
    Element(NodeId),
    /// An anonymous box holding collapsed text in its parent's style.
    Text { text: String, style: TextStyle },
}

/// One box: its content, Taffy style, children, and Taffy's results.
pub(crate) struct LayoutBox {
    pub(crate) content: Content,
    pub(crate) style: Style,
    pub(crate) children: Vec<usize>,
    pub(crate) cache: Cache,
    pub(crate) layout: Layout,
}

impl LayoutBox {
    fn new(content: Content, style: Style) -> Self {
        Self {
            content,
            style,
            children: Vec::new(),
            cache: Cache::new(),
            layout: Layout::with_order(0),
        }
    }
}

/// Builds the boxes under `root`; box 0 is the root's.
pub(crate) fn build(arena: &Arena, used: &HashMap<NodeId, Used>, root: NodeId) -> Vec<LayoutBox> {
    let mut boxes = Vec::new();
    add_element(arena, used, root, &mut boxes);
    boxes
}

fn add_element(
    arena: &Arena,
    used: &HashMap<NodeId, Used>,
    node: NodeId,
    boxes: &mut Vec<LayoutBox>,
) -> Option<usize> {
    let style = &used[&node];
    if style.layout.display == Display::None {
        return None;
    }
    let index = boxes.len();
    boxes.push(LayoutBox::new(Content::Element(node), style.layout.clone()));
    let mut run = String::new();
    for child in arena.children(node) {
        match &arena.get(child, "build").kind {
            Kind::Text(text) => run.push_str(text),
            Kind::Element(_) => {
                add_text(&mut run, style.text, index, boxes);
                if let Some(child_box) = add_element(arena, used, child, boxes) {
                    boxes[index].children.push(child_box);
                }
            }
        }
    }
    add_text(&mut run, style.text, index, boxes);
    Some(index)
}

fn add_text(run: &mut String, style: TextStyle, parent: usize, boxes: &mut Vec<LayoutBox>) {
    let text = collapse(run);
    run.clear();
    if text.is_empty() {
        return;
    }
    let index = boxes.len();
    let box_style = Style {
        display: Display::Block,
        ..Style::default()
    };
    boxes.push(LayoutBox::new(Content::Text { text, style }, box_style));
    boxes[parent].children.push(index);
}

/// `white-space: normal`: each run of spaces, tabs and line breaks is
/// one space, and none is kept at either end of the run.
pub(crate) fn collapse(text: &str) -> String {
    text.split([' ', '\t', '\n', '\r', '\u{c}'])
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::collapse;

    #[test]
    fn whitespace_runs_collapse_to_one_space_and_the_ends_are_trimmed() {
        // Given
        let text = "\n    Hello,\t  vapors \n";

        // When
        let collapsed = collapse(text);

        // Then
        assert_eq!(collapsed, "Hello, vapors");
    }

    #[test]
    fn a_no_break_space_is_not_collapsed() {
        // Given
        let text = "a\u{a0}\u{a0}b";

        // When
        let collapsed = collapse(text);

        // Then
        assert_eq!(collapsed, "a\u{a0}\u{a0}b");
    }
}
