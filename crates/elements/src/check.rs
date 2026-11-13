//! The refusals that follow from the tree itself: tags and listeners
//! the elements crate does not implement yet, for elements under the
//! root, and right-to-left text, for text nodes. Each is a node's own,
//! read from that node alone, so a node is checked without reading its
//! children. Asked of the tree on each update, so removing the node
//! removes the refusal.

use std::fmt::Write;

use vapors_dom::NodeId;

use crate::arena::Arena;
pub(crate) use crate::check_inputs::is_type;
use crate::error::ElementError;

use crate::check_inputs::{disabled_value, input_refusal};
use crate::node::Kind;
use crate::tags;

/// Every element under `root`, root first, in document order.
pub(crate) fn elements(arena: &Arena, root: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if arena.get(node, "elements").element().is_none() {
            continue;
        }
        out.push(node);
        stack.extend(arena.children(node).into_iter().rev());
    }
    out
}

/// Every node under `root`, root first, in document order.
pub(crate) fn nodes(arena: &Arena, root: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        out.push(node);
        stack.extend(arena.children(node).into_iter().rev());
    }
    out
}

/// The refusals for tags, listeners and text under `root`.
pub(crate) fn check_tree(
    arena: &Arena,
    root: NodeId,
    declared: &std::collections::BTreeSet<String>,
) -> Vec<ElementError> {
    nodes(arena, root)
        .into_iter()
        .flat_map(|node| check_node(arena, node, declared))
        .collect()
}

/// The refusals for `node` alone: an element's (`check_element`), or
/// a text node's right-to-left text.
pub(crate) fn check_node(
    arena: &Arena,
    node: NodeId,
    declared: &std::collections::BTreeSet<String>,
) -> Vec<ElementError> {
    match &arena.get(node, "check").kind {
        Kind::Element(_) => check_element(arena, node, declared),
        Kind::Text(text) => right_to_left_text(arena, node, text).into_iter().collect(),
    }
}

/// The refusals for `node`'s tag, attributes and listeners, and the
/// calls refused on it (`dom_keyed`).
pub(crate) fn check_element(
    arena: &Arena,
    node: NodeId,
    declared: &std::collections::BTreeSet<String>,
) -> Vec<ElementError> {
    let Kind::Element(element) = &arena.get(node, "check").kind else {
        return Vec::new();
    };
    let mut errors: Vec<ElementError> = tags::unsupported(&element.tag)
        .map(ElementError::UnsupportedTag)
        .into_iter()
        .collect();
    errors.extend(attribute_refusals(arena, node, element));
    errors.extend(element.refused.iter().cloned().map(ElementError::Refused));
    for (event, _) in &element.listeners {
        if let Some(refusal) = tags::unfired(event).filter(|_| !declared.contains(event)) {
            errors.push(ElementError::UnfiredEvent(format!(
                "{}: {refusal}",
                describe(arena, node)
            )));
        }
    }
    errors
}

/// A refusal when `text`, the text node `node` holds, has right-to-left
/// text, which is not laid out right to left yet; named for its parent.
fn right_to_left_text(arena: &Arena, node: NodeId, text: &str) -> Option<ElementError> {
    let found = text.chars().find(|c| crate::bidi::right_to_left(*c))?;
    let named = arena.get(node, "check").parent.unwrap_or(node);
    Some(ElementError::UnsupportedAttribute(format!(
        "{}: its text holds {found:?} (U+{:04X}), from a script written right to left; vapors-elements lays text out left to right only (not scheduled in docs/F2c.md)",
        describe(arena, named),
        u32::from(found)
    )))
}

/// The attributes of `element` the tree refuses: a `disabled` that
/// would mean something other than it says.
fn attribute_refusals(
    arena: &Arena,
    node: NodeId,
    element: &crate::node::Element,
) -> Vec<ElementError> {
    let mut found = Vec::new();
    found.extend(disabled_value(
        &element.tag,
        element.attributes.get("disabled"),
    ));
    found.extend(crate::focus::tabindex_refusal(
        element.attributes.get("tabindex"),
    ));
    found.extend(input_refusal(element));
    found
        .into_iter()
        .map(|refusal| {
            ElementError::UnsupportedAttribute(format!("{}: {refusal}", describe(arena, node)))
        })
        .collect()
}

/// Each element's `style` attribute that does not parse, naming the
/// element and the line and column in the attribute. A tree with one
/// cannot be styled, so the update stops at these.
pub(crate) fn style_attribute_refusals(arena: &Arena, root: NodeId) -> Vec<ElementError> {
    elements(arena, root)
        .into_iter()
        .filter_map(|node| style_attribute_refusal(arena, node))
        .collect()
}

/// The refusal for `node`'s `style` attribute, when it does not parse.
pub(crate) fn style_attribute_refusal(arena: &Arena, node: NodeId) -> Option<ElementError> {
    let element = arena.get(node, "style_attribute_refusals").element()?;
    let error = vapors_css::parse_inline(element.attributes.get("style")?).err()?;
    Some(ElementError::UnsupportedAttribute(format!(
        "{}: its style attribute {}:{}: {}",
        describe(arena, node),
        error.position.line,
        error.position.column,
        error.message
    )))
}

/// The element as errors name it: `<div class="panel">`.
pub(crate) fn describe(arena: &Arena, node: NodeId) -> String {
    let Some(element) = arena.get(node, "describe").element() else {
        return format!("text node {node:?}");
    };
    let mut out = format!("<{}", element.tag);
    for name in ["id", "class"] {
        if let Some(value) = element.attributes.get(name) {
            let _ = write!(out, " {name}=\"{value}\"");
        }
    }
    out.push('>');
    out
}

/// Whether the element's tag is implemented; an unimplemented tag is
/// refused once, by `check_tree`, and its style is not read.
pub(crate) fn implemented(arena: &Arena, node: NodeId) -> bool {
    arena
        .get(node, "implemented")
        .element()
        .is_some_and(|element| tags::unsupported(&element.tag).is_none())
}
