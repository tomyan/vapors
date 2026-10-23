//! The refusals that follow from the tree itself: tags and listeners
//! the elements crate does not implement yet, for elements under the
//! root. Asked of the tree on each update, so removing the element
//! removes the refusal.

use std::fmt::Write;

use vapors_dom::NodeId;

use crate::arena::Arena;
use crate::error::ElementError;
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

/// The refusals for tags and listeners under `root`.
pub(crate) fn check_tree(arena: &Arena, root: NodeId) -> Vec<ElementError> {
    let mut errors = Vec::new();
    for node in elements(arena, root) {
        let Kind::Element(element) = &arena.get(node, "check").kind else {
            continue;
        };
        if let Some(refusal) = tags::unsupported(&element.tag) {
            errors.push(ElementError::UnsupportedTag(refusal));
        }
        if element.attributes.contains_key("style") {
            errors.push(ElementError::UnsupportedAttribute(format!(
                "{}: a `style` attribute is not applied by vapors-elements yet: inline styles arrive in F2c S5 (docs/F2c.md); move the declarations into the component's <style> or a stylesheet",
                describe(arena, node)
            )));
        }
        for (event, _) in &element.listeners {
            if let Some(refusal) = tags::unfired(event) {
                errors.push(ElementError::UnfiredEvent(format!(
                    "{}: {refusal}",
                    describe(arena, node)
                )));
            }
        }
    }
    errors
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
