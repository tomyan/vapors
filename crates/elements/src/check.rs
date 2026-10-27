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
        errors.extend(attribute_refusals(arena, node, element));
        errors.extend(right_to_left_text(arena, node));
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

/// A refusal when a text child of `node` holds right-to-left text,
/// which is not laid out right to left yet.
fn right_to_left_text(arena: &Arena, node: NodeId) -> Option<ElementError> {
    let found = arena.children(node).into_iter().find_map(|child| {
        match &arena.get(child, "check").kind {
            Kind::Text(text) => text.chars().find(|c| crate::bidi::right_to_left(*c)),
            Kind::Element(_) => None,
        }
    })?;
    Some(ElementError::UnsupportedAttribute(format!(
        "{}: its text holds {found:?} (U+{:04X}), from a script written right to left; vapors-elements lays text out left to right only (not scheduled in docs/F2c.md)",
        describe(arena, node),
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
        .filter_map(|node| {
            let element = arena.get(node, "style_attribute_refusals").element()?;
            let error = vapors_css::parse_inline(element.attributes.get("style")?).err()?;
            Some(ElementError::UnsupportedAttribute(format!(
                "{}: its style attribute {}:{}: {}",
                describe(arena, node),
                error.position.line,
                error.position.column,
                error.message
            )))
        })
        .collect()
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

/// A `disabled` attribute disables by being present, as in HTML, so
/// `disabled="false"` disables too; a `.wisp` that sets it from an
/// expression would get exactly that. Refused where `disabled` has an
/// effect, a button or an input (elsewhere HTML ignores it), until the
/// tree can remove an attribute (a `Dom` change).
fn disabled_value(tag: &str, value: Option<&String>) -> Option<String> {
    if !matches!(tag, "button" | "input") {
        return None;
    }
    match value.map(String::as_str) {
        Some("false") => Some(format!(
            "<{tag} disabled=\"false\"> is disabled: an HTML boolean attribute disables by being present; leave it out to enable, since the tree cannot remove an attribute yet"
        )),
        _ => None,
    }
}

/// Whether `element`'s `type` attribute is `kind`, compared as HTML
/// compares it (ASCII case-insensitively); an `input` without one is
/// `text`.
pub(crate) fn is_type(element: &crate::node::Element, kind: &str) -> bool {
    element
        .attributes
        .get("type")
        .map_or("text", String::as_str)
        .eq_ignore_ascii_case(kind)
}

/// The input types built and where the rest arrive (3-E3).
const INPUT_TYPES: &[(&str, Option<&str>)] = &[
    ("range", None),
    ("checkbox", Some("3-E3b")),
    ("number", Some("3-E3c")),
    ("text", Some("3-E6, text editing")),
];

/// An `input` of a type not built yet, naming the slice; and a `value`
/// property of a shape no input's value takes.
fn input_refusal(element: &crate::node::Element) -> Option<String> {
    if element.tag != "input" {
        return None;
    }
    let built = INPUT_TYPES
        .iter()
        .find(|(kind, _)| is_type(element, kind))
        .map_or(Some("no slice yet"), |(_, slice)| *slice);
    if let Some(slice) = built {
        let kind = element
            .attributes
            .get("type")
            .map_or("text", String::as_str);
        return Some(format!(
            "<input type=\"{kind}\"> is not implemented by vapors-elements yet: it arrives in {slice} (docs/design/03-events-focus-input-ime.md); the types built are {}",
            INPUT_TYPES
                .iter()
                .filter(|(_, s)| s.is_none())
                .map(|(k, _)| *k)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    match element.properties.get("value") {
        Some(vapors_dom::PropertyValue::Bool(_)) => Some(
            "its `value` property is a boolean; an input's value is a number or a text (set_property with PropertyValue::Number or Text)".to_owned(),
        ),
        _ => None,
    }
}
