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
/// a text node's right-to-left text, named for its parent. The checks
/// are given the node's own data and a name for messages, not the
/// arena, so none can read its children: a parent whose children
/// changed is checked in constant time
/// (`docs/perf/insert-style-2026-11-13.md`).
pub(crate) fn check_node(
    arena: &Arena,
    node: NodeId,
    declared: &std::collections::BTreeSet<String>,
) -> Vec<ElementError> {
    let found = arena.get(node, "check");
    match &found.kind {
        Kind::Element(element) => check_element(element, &|| describe(arena, node), declared),
        Kind::Text(text) => {
            let parent = found.parent.unwrap_or(node);
            right_to_left_text(text, &|| describe(arena, parent))
                .into_iter()
                .collect()
        }
    }
}

/// The refusals for `element`'s tag, attributes and listeners, and the
/// calls refused on it (`dom_keyed`); `named` describes it in messages.
fn check_element(
    element: &crate::node::Element,
    named: &dyn Fn() -> String,
    declared: &std::collections::BTreeSet<String>,
) -> Vec<ElementError> {
    let mut errors: Vec<ElementError> = tags::unsupported(&element.tag)
        .map(ElementError::UnsupportedTag)
        .into_iter()
        .collect();
    errors.extend(attribute_refusals(element, named));
    errors.extend(element.refused.iter().cloned().map(ElementError::Refused));
    for (event, _) in &element.listeners {
        if let Some(refusal) = tags::unfired(event).filter(|_| !declared.contains(event)) {
            errors.push(ElementError::UnfiredEvent(format!(
                "{}: {refusal}",
                named()
            )));
        }
    }
    errors
}

/// A refusal when `text`, a text node's, has right-to-left text, which
/// is not laid out right to left yet; `named` describes the node's
/// parent. One per text node: an element with two such children is
/// named twice.
fn right_to_left_text(text: &str, named: &dyn Fn() -> String) -> Option<ElementError> {
    let found = text.chars().find(|c| crate::bidi::right_to_left(*c))?;
    Some(ElementError::UnsupportedAttribute(format!(
        "{}: its text holds {found:?} (U+{:04X}), from a script written right to left; vapors-elements lays text out left to right only (not scheduled in docs/F2c.md)",
        named(),
        u32::from(found)
    )))
}

/// The attributes of `element` the tree refuses: a `disabled` that
/// would mean something other than it says.
fn attribute_refusals(
    element: &crate::node::Element,
    named: &dyn Fn() -> String,
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
        .map(|refusal| ElementError::UnsupportedAttribute(format!("{}: {refusal}", named())))
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

/// The element as errors name it: `<div class="panel">`; a text node
/// by its id under its parent.
pub(crate) fn describe(arena: &Arena, node: NodeId) -> String {
    let found = arena.get(node, "describe");
    let Some(element) = found.element() else {
        return match found.parent {
            Some(parent) => format!("text node {node:?} under {}", describe(arena, parent)),
            None => format!("text node {node:?}"),
        };
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

/// Whether `node` is a scene component's anchor (`tags::is_scene_anchor`).
pub(crate) fn is_scene_anchor(arena: &Arena, node: NodeId) -> bool {
    arena
        .get(node, "is_scene_anchor")
        .element()
        .is_some_and(|element| tags::is_scene_anchor(&element.tag))
}
