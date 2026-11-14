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

/// `css` classified after `scope::scoped` scoped it for scope `s`, as
/// the runtime holds a component's sheet (the scoped selector is in
/// the rule, not in its text).
pub fn classified_scoped(css: &str, facts: &KeyFacts<'_>) -> Match {
    let sheet = vapors_css::parse(css).unwrap_or_else(|e| panic!("{css}: {e}"));
    let scoped = vapors_css::scope::scoped(sheet, "data-wisp", "s");
    classify(&scoped.rules[0], facts)
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
