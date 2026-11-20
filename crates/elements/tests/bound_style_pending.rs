//! `Document::bind_style` refuses every call until the slot lands
//! (decision 2026-11-20-style-bindings, Status): loud, never silent.

use crate::support::{document, viewport};
use vapors_dom::{BoundProperty, BoundValue, Dom, StyleKey, Winners};

static BAR: StyleKey = StyleKey {
    site: "crates/x/bars.wisp:3:5",
    scope: None,
    tag: "div",
    classes: &[],
    toggles: &[],
    attributes: &[],
    style: None,
    bound: &[BoundProperty::Height],
    winners: Winners::Unanalysed,
};

#[test]
fn a_binding_is_refused_by_every_update_naming_the_site() {
    let mut doc = document();
    let bar = doc.create_keyed_element(&BAR);
    doc.append(doc.body(), bar);
    doc.bind_style(bar, BoundProperty::Height, BoundValue::Length(4.0));
    let Err(errors) = doc.update(viewport(100.0, 100.0)) else {
        panic!("expected a refusal");
    };
    let why: String = errors.iter().map(ToString::to_string).collect();
    assert!(why.contains("crates/x/bars.wisp:3:5"), "{why}");
    assert!(why.contains("does not apply style bindings yet"), "{why}");
}
