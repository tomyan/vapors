//! The check of `honoured.rs`'s table against one element's computed
//! style: a deferred or off longhand set other than its initial value
//! is refused where it was declared.

use vapors_css::cascade::ComputedStyle;
use vapors_css::property::PropertyId;

use super::honoured::{Deferred, Off, Read, TABLE};
use super::values::{Refusal, Styles, later};

/// The deferred properties of `styles.own` that are not at their
/// initial value in `initial`, each reported at the element that
/// declared it. `TABLE` is in vapors-css's table order (its test holds
/// them together), so each entry's id is its position: nothing is
/// looked up by name, and nothing is allocated when nothing is refused.
/// Only the longhands a declaration reached are visited
/// (`ComputedStyle::declared_ids`, about ten per element rather than
/// the table); debug builds check that the rest are at their initial
/// value.
pub(crate) fn check_honoured(styles: Styles<'_>, initial: &ComputedStyle) -> Vec<Refusal> {
    debug_assert_eq!(
        TABLE.len(),
        PropertyId::all().count(),
        "crates/elements/src/style/honoured.rs TABLE and vapors-css's property table differ in length; add or remove entries to match (cargo test -p vapors-elements --lib every_css_longhand_has_exactly_one_treatment)"
    );
    #[cfg(debug_assertions)]
    assert_undeclared_at_initial(styles, initial);
    styles
        .own
        .declared_ids()
        .filter_map(|id| match TABLE[id.index()].1 {
            Deferred(slice) => in_use(styles, initial, id, || {
                format!(
                    "is not implemented by vapors-elements yet: it {}",
                    later(slice)
                )
            }),
            Off(why) => in_use(styles, initial, id, || why.to_owned()),
            Read => None,
        })
        .collect()
}

/// The rule the skip relies on: a deferred or off longhand no
/// declaration reached computes to its initial value.
#[cfg(debug_assertions)]
fn assert_undeclared_at_initial(styles: Styles<'_>, initial: &ComputedStyle) {
    for ((_, treatment), id) in TABLE.iter().zip(PropertyId::all()) {
        if matches!(treatment, Read) || styles.own.declared(id) {
            continue;
        }
        assert!(
            styles.own.get(id) == initial.get(id),
            "{}: no declaration reached it, yet its computed value `{}` is not the initial `{}` (vapors-css `Declared`, crates/css/src/cascade/declared.rs); reproduce with the failing test",
            id.name(),
            styles.own.get(id).to_css(),
            initial.get(id).to_css()
        );
    }
}

/// A refusal for `id` when it is set other than its initial value.
fn in_use(
    styles: Styles<'_>,
    initial: &ComputedStyle,
    id: PropertyId,
    reason: impl FnOnce() -> String,
) -> Option<Refusal> {
    if styles.own.get(id) == initial.get(id) {
        return None;
    }
    styles.refusal(id, reason())
}
