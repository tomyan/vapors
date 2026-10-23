//! What vapors-elements does with each property of vapors-css's table.
//! One entry per longhand; a test holds this table and the CSS table
//! together, so a new CSS property fails until it is decided here.
//! A deferred property whose computed value is not its initial value
//! is refused: it would otherwise be parsed, cascaded and silently
//! ignored.

use vapors_css::cascade::ComputedStyle;
use vapors_css::property::PropertyId;

use super::values::{Refusal, Styles, later};

/// What the tree does with a property.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Treatment {
    /// Read by layout, text or paint (`box_style`, `text_style`, `paint`).
    Read,
    /// Has no effect while another property is at a value this slice
    /// requires; the reason names it.
    Inert(&'static str),
    /// Not implemented; arrives in the named slice.
    Deferred(&'static str),
}

use Treatment::{Deferred, Inert, Read};

/// Every longhand, sorted by name, with its treatment.
pub(crate) const TABLE: &[(&str, Treatment)] = &[
    ("align-content", Read),
    ("align-items", Read),
    ("align-self", Read),
    ("animation-delay", Deferred("a slice after F2c")),
    ("animation-direction", Deferred("a slice after F2c")),
    ("animation-duration", Deferred("a slice after F2c")),
    ("animation-fill-mode", Deferred("a slice after F2c")),
    ("animation-iteration-count", Deferred("a slice after F2c")),
    ("animation-name", Deferred("a slice after F2c")),
    ("animation-play-state", Deferred("a slice after F2c")),
    ("animation-timing-function", Deferred("a slice after F2c")),
    ("aspect-ratio", Read),
    ("background-color", Read),
    ("border-bottom-color", Read),
    ("border-bottom-left-radius", Read),
    ("border-bottom-right-radius", Read),
    ("border-bottom-style", Read),
    ("border-bottom-width", Read),
    ("border-left-color", Read),
    ("border-left-style", Read),
    ("border-left-width", Read),
    ("border-right-color", Read),
    ("border-right-style", Read),
    ("border-right-width", Read),
    ("border-top-color", Read),
    ("border-top-left-radius", Read),
    ("border-top-right-radius", Read),
    ("border-top-style", Read),
    ("border-top-width", Read),
    (
        "bottom",
        Inert("has no effect while position is static, which F2c S1 requires"),
    ),
    ("box-shadow", Read),
    ("box-sizing", Read),
    ("color", Read),
    ("column-gap", Read),
    (
        "container-name",
        Deferred("no slice yet: @container needs layout-time restyle"),
    ),
    (
        "container-type",
        Deferred("no slice yet: @container needs layout-time restyle"),
    ),
    ("cursor", Read),
    ("display", Read),
    ("filter", Deferred("F3")),
    ("flex-basis", Read),
    ("flex-direction", Read),
    ("flex-grow", Read),
    ("flex-shrink", Read),
    ("flex-wrap", Read),
    ("font-family", Read),
    ("font-feature-settings", Deferred("F2c S6")),
    ("font-size", Read),
    ("font-style", Deferred("F2c S6")),
    ("font-variation-settings", Deferred("F2c S6")),
    ("font-weight", Read),
    ("grid-auto-columns", Read),
    ("grid-auto-flow", Read),
    ("grid-auto-rows", Read),
    ("grid-column-end", Read),
    ("grid-column-start", Read),
    ("grid-row-end", Read),
    ("grid-row-start", Read),
    ("grid-template-columns", Read),
    ("grid-template-rows", Read),
    ("height", Read),
    ("justify-content", Read),
    ("justify-items", Read),
    ("justify-self", Read),
    (
        "left",
        Inert("has no effect while position is static, which F2c S1 requires"),
    ),
    ("letter-spacing", Deferred("F2c S6")),
    ("line-height", Read),
    ("margin-bottom", Read),
    ("margin-left", Read),
    ("margin-right", Read),
    ("margin-top", Read),
    ("max-height", Read),
    ("max-width", Read),
    ("min-height", Read),
    ("min-width", Read),
    ("opacity", Deferred("F3")),
    ("overflow-x", Deferred("F2c S8")),
    ("overflow-y", Deferred("F2c S8")),
    ("padding-bottom", Read),
    ("padding-left", Read),
    ("padding-right", Read),
    ("padding-top", Read),
    ("pointer-events", Read),
    ("position", Deferred("F2c S8")),
    (
        "right",
        Inert("has no effect while position is static, which F2c S1 requires"),
    ),
    ("row-gap", Read),
    ("text-align", Deferred("F2c S6")),
    ("text-decoration", Deferred("F2c S6")),
    (
        "top",
        Inert("has no effect while position is static, which F2c S1 requires"),
    ),
    ("transform", Deferred("a slice after F2c")),
    ("transition-delay", Deferred("a slice after F2c")),
    ("transition-duration", Deferred("a slice after F2c")),
    ("transition-property", Deferred("a slice after F2c")),
    ("transition-timing-function", Deferred("a slice after F2c")),
    ("user-select", Deferred("F4")),
    ("visibility", Read),
    ("white-space", Read),
    ("width", Read),
    ("z-index", Deferred("F2c S8")),
];

/// The deferred properties of `styles.own` that are not at their
/// initial value in `initial`, each reported at the element that
/// declared it.
pub(crate) fn check_honoured(styles: Styles<'_>, initial: &ComputedStyle) -> Vec<Refusal> {
    TABLE
        .iter()
        .filter_map(|(name, treatment)| match treatment {
            Deferred(slice) => deferred_in_use(styles, initial, name, slice),
            Read | Inert(_) => None,
        })
        .collect()
}

fn deferred_in_use(
    styles: Styles<'_>,
    initial: &ComputedStyle,
    name: &'static str,
    slice: &str,
) -> Option<Refusal> {
    let id = PropertyId::from_name(name)
        .unwrap_or_else(|| panic!("`{name}` is not in vapors-css's table"));
    if styles.own.get(id) == initial.get(id) {
        return None;
    }
    let reason = format!(
        "is not implemented by vapors-elements yet: it {}",
        later(slice)
    );
    styles.refusal(name, reason)
}

#[cfg(test)]
#[path = "honoured_tests.rs"]
mod tests;
