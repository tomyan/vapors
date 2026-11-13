//! Shared by `analysis_classify.rs` and `analysis_classify_scoped.rs`:
//! one key, a rule classified against it, and a `Depends` built in place.

#![allow(dead_code)] // each test binary uses a subset

use vapors_css::analysis::{Depends, KeyFacts, Match, classify};

/// `<button class="card" class:open={..} type="text" id={..} data-wisp="s">`
pub const CARD: KeyFacts<'static> = KeyFacts {
    tag: "button",
    scope_attribute: "data-wisp",
    scope: Some("s"),
    classes: &["card"],
    toggles: &["open"],
    style: Some("color: red"),
    literal_attributes: &[("type", "Text")],
};

/// `css` with its subject scoped as `scope::scoped` scopes a component's
/// sheet for scope `s`.
pub fn scoped(css: &str) -> String {
    let sheet = vapors_css::parse(css).unwrap_or_else(|e| panic!("{css}: {e}"));
    vapors_css::scope::scoped(sheet, "data-wisp", "s").to_css()
}

pub fn classified(css: &str, facts: &KeyFacts<'_>) -> Match {
    let sheet = vapors_css::parse(css).unwrap_or_else(|e| panic!("{css}: {e}"));
    classify(&sheet.rules[0], facts)
}

pub fn depends(set: impl FnOnce(&mut Depends)) -> Match {
    let mut depends = Depends::default();
    set(&mut depends);
    Match::Conditional(depends)
}

