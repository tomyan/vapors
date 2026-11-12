//! What a mirror cannot show truthfully (R6): a node whose role ARIA
//! requires a state for that the tree does not carry. A browser reads a
//! default for the missing state (a slider with no value is 0 of 100),
//! so the node is refused, naming what is missing and where it comes
//! from, rather than shown wrong.

use crate::a11y_mirror::AxMirrorNode;

/// Roles that require `aria-checked`.
const NEEDS_CHECKED: [&str; 4] = ["radio", "switch", "menuitemcheckbox", "menuitemradio"];

/// What `record` lacks that its role requires, if anything.
pub(crate) fn unexposable(record: &AxMirrorNode) -> Option<String> {
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
        role if NEEDS_CHECKED.contains(&role) && record.states.checked.is_none() => {
            "aria-checked: set it on the element"
        }
        _ => return None,
    };
    Some(needs.to_owned())
}
