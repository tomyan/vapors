//! What a mirror cannot show truthfully (R6): a node whose role ARIA
//! requires a state for that the tree does not carry, or whose state is
//! one the tree cannot write (`mixed`). A browser reads a default for the
//! missing state (a slider with no value is 0 of 100), so the node is
//! refused, naming what is missing and where it comes from, rather than
//! shown wrong.

use vapors_dom::NodeId;

use crate::a11y_mirror::AxMirrorNode;
use crate::document::Document;

/// Roles that require `aria-checked`.
const NEEDS_CHECKED: [&str; 5] = [
    "checkbox",
    "radio",
    "switch",
    "menuitemcheckbox",
    "menuitemradio",
];

/// What `record` lacks that its role requires, if anything.
pub(crate) fn unexposable(doc: &Document, record: &AxMirrorNode) -> Option<String> {
    if let Some(mixed) = mixed_state(doc, record) {
        return Some(mixed);
    }
    let needs = match record.role.as_str() {
        "heading" => "aria-level, which the tree does not carry yet (slice A6)",
        "meter" => {
            "aria-valuenow, which the tree carries for slider and spinbutton only (slice A6)"
        }
        "scrollbar" => "aria-controls and aria-valuenow, which the tree does not carry (slice A6)",
        "combobox" => {
            "aria-controls as an element reference, which the tree does not carry (slice A6)"
        }
        "slider" if record.value.is_none() => {
            "a value: set `value` or aria-valuenow on the element"
        }
        "separator" if record.focusable && !has(doc, record.node, "aria-valuenow") => {
            "aria-valuenow: a separator that takes focus is a widget; set it on the element, or drop its tabindex"
        }
        role if NEEDS_CHECKED.contains(&role) && record.states.checked.is_none() => {
            "aria-checked: set it on the element"
        }
        _ => return None,
    };
    Some(needs.to_owned())
}

/// A `mixed` state the tree read as none: it writes `true` or `false`
/// only, so the mirror would show the state missing.
fn mixed_state(doc: &Document, record: &AxMirrorNode) -> Option<String> {
    let unread = [
        ("aria-checked", record.states.checked),
        ("aria-pressed", record.states.pressed),
    ];
    unread
        .into_iter()
        .find(|&(attribute, state)| {
            state.is_none() && doc.attribute(record.node, attribute) == Some("mixed")
        })
        .map(|(attribute, _)| {
            format!("{attribute} as true or false: the tree does not carry mixed (slice A6)")
        })
}

fn has(doc: &Document, node: NodeId, attribute: &str) -> bool {
    doc.attribute(node, attribute).is_some()
}
