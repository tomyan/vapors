//! A node's record as its mirror element's attributes and text (R6,
//! `docs/plans/r6.md`, "Element mapping"). Every attribute the mirror
//! may set is in the list, as a value or as none, in one order, so a
//! state that went is removed from the element on the next record and
//! two records' lists compare attribute by attribute
//! (`a11y_mirror_write.rs`). The bounds are `position: absolute` from
//! the parent element's padding box, as the record holds them; a
//! scroll container's element scrolls (`overflow: auto`) over the
//! record's extent, which the page's rule for `data-scroll` sizes from
//! the custom properties here. The live roles get `aria-live="off"`
//! until slice A5 adds live regions on purpose.

use std::fmt::Write;

use vapors_a11y::AxMirrorNode;

/// An attribute's name and its value, or none to remove it.
pub(crate) type Attribute = (&'static str, Option<String>);

/// Roles live in every browser unasked, silenced until A5.
const LIVE: [&str; 5] = ["status", "log", "alert", "timer", "marquee"];

/// Roles whose value is `aria-valuenow` in a range.
const RANGED: [&str; 2] = ["slider", "spinbutton"];

/// Roles whose value is the element's text, as Chromium computes a
/// non-native text field's value.
const TEXT_VALUED: [&str; 3] = ["textbox", "searchbox", "combobox"];

/// `node`'s attributes, every one the mirror may set.
///
/// # Errors
/// A ranged role without a value or a range, or a value on a role the
/// mirror has no attribute for: the document's take should have
/// refused the node, so the mirror does not guess.
pub(crate) fn attributes(node: &AxMirrorNode) -> Result<Vec<Attribute>, String> {
    let role = node.role.as_str();
    let states = &node.states;
    let mut attributes = vec![
        ("role", Some(node.role.clone())),
        ("data-node", Some(node.node.0.to_string())),
        ("style", Some(style(node))),
        ("data-scroll", node.scroll.map(|_| String::new())),
        ("aria-label", non_empty(&node.name)),
        ("aria-description", node.description.clone()),
        ("aria-disabled", states.disabled.then(|| "true".to_owned())),
        tri("aria-checked", states.checked),
        tri("aria-expanded", states.expanded),
        tri("aria-pressed", states.pressed),
        ("aria-selected", selected(role, states.selected)),
        ("tabindex", node.focusable.then(|| "-1".to_owned())),
        ("aria-live", LIVE.contains(&role).then(|| "off".to_owned())),
    ];
    attributes.extend(ranged(node)?);
    Ok(attributes)
}

/// The element's text: a text field's value, or none.
pub(crate) fn text(node: &AxMirrorNode) -> &str {
    if TEXT_VALUED.contains(&node.role.as_str()) {
        node.value.as_deref().unwrap_or("")
    } else {
        ""
    }
}

/// The element's place in its parent's padding box (in the viewport,
/// fixed, for a hoisted node), and its scroll extent when it scrolls.
fn style(node: &AxMirrorNode) -> String {
    let position = if node.hoisted { "fixed" } else { "absolute" };
    let bounds = &node.bounds;
    let mut style = format!(
        "position:{position};left:{}px;top:{}px;width:{}px;height:{}px",
        bounds.x, bounds.y, bounds.width, bounds.height
    );
    if let Some(scroll) = node.scroll {
        let [width, height] = scroll.extent;
        // writing to a String cannot fail
        let _ = write!(
            style,
            ";overflow:auto;--vapors-scroll-w:{width}px;--vapors-scroll-h:{height}px"
        );
    }
    style
}

fn non_empty(text: &str) -> Option<String> {
    (!text.is_empty()).then(|| text.to_owned())
}

fn tri(name: &'static str, state: Option<bool>) -> Attribute {
    (name, state.map(|state| state.to_string()))
}

/// `aria-selected`: `true` when selected; `false` on an `option`
/// without it, where ARIA requires the attribute.
fn selected(role: &str, selected: bool) -> Option<String> {
    if selected {
        Some("true".to_owned())
    } else if role == "option" {
        Some("false".to_owned())
    } else {
        None
    }
}

/// `aria-valuenow`, `aria-valuemin` and `aria-valuemax` for a ranged
/// role; none for the others.
fn ranged(node: &AxMirrorNode) -> Result<[Attribute; 3], String> {
    let role = node.role.as_str();
    if !RANGED.contains(&role) {
        if node.value.is_some() && !TEXT_VALUED.contains(&role) {
            return Err(format!(
                "the a11y record of {role} \"{}\" carries a value, and the mirror has no \
                 attribute for a {role}'s value: a11y_mirror_attrs.rs names the roles with one",
                node.name
            ));
        }
        return Ok([
            ("aria-valuenow", None),
            ("aria-valuemin", None),
            ("aria-valuemax", None),
        ]);
    }
    let missing = |what: &str| {
        format!(
            "the a11y record of {role} \"{}\" has no {what}, which a reader would read as a \
             default: the document's take should have refused it (a11y_mirror_roles.rs)",
            node.name
        )
    };
    let value = node.value.clone().ok_or_else(|| missing("value"))?;
    let range = node.range.ok_or_else(|| missing("range"))?;
    Ok([
        ("aria-valuenow", Some(value)),
        ("aria-valuemin", Some(range.min.to_string())),
        ("aria-valuemax", Some(range.max.to_string())),
    ])
}
