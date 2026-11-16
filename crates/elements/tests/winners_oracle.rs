//! Debug builds hold a key's compiled winners against the cascade's
//! (review of #429, M1): an element computed from a key whose base
//! winner names another declaration than the cascade took panics,
//! naming the element and the property.

use vapors_css::property::PropertyId;
use vapors_dom::{
    BaseWinner, DeclarationRef, Dom, KeyWinners, PropertyRef, RuleRef, SheetRef, StyleKey, Winners,
};
use vapors_elements::{Document, Viewport};
use vapors_text::FontSystem;

const OWN: &str = "cards/card.wisp";
const CARD: &str = ".card { color: red; }";
const VIEWPORT: Viewport = Viewport {
    width: 200.0,
    height: 100.0,
    scale: 1.0,
};

/// A key whose winners say `color` is won by declaration `declaration`
/// of `.card`, every user-agent rule left to run time (as conditional)
/// so the test names no other winner.
fn key(declaration: u32) -> &'static StyleKey {
    let color = PropertyId::from_name("color").expect("a longhand").index();
    let base = vec![BaseWinner {
        property: PropertyRef::Longhand(u16::try_from(color).expect("fits")),
        declaration: DeclarationRef::Rule {
            rule: RuleRef { sheet: 1, rule: 0 },
            declaration,
        },
    }];
    let winners = Box::leak(Box::new(KeyWinners {
        sheets: &[(SheetRef::UserAgent, 0), (SheetRef::Scope(OWN), 0)],
        base: base.leak(),
        conditional: user_agent_rules().leak(),
        shareable: true,
    }));
    Box::leak(Box::new(StyleKey {
        site: "cards/card.wisp:3:1",
        scope: Some(OWN),
        tag: "span",
        classes: &["card"],
        toggles: &[],
        attributes: &[],
        style: None,
        winners: Winners::Compiled(winners),
    }))
}

fn user_agent_rules() -> Vec<RuleRef> {
    let count = vapors_css::user_agent::sheet().rules.len();
    let rule = |rule| RuleRef {
        sheet: 0,
        rule: u32::try_from(rule).expect("fits"),
    };
    (0..count).map(rule).collect()
}

fn updated(key: &'static StyleKey) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_style(OWN, CARD);
    let body = doc.body();
    let node = doc.create_keyed_element(key);
    doc.insert_before(body, node, None);
    if let Some(errors) = doc.update(VIEWPORT).err() {
        panic!("{errors:?}");
    }
}

#[test]
fn winners_that_name_the_cascades_declaration_pass() {
    updated(key(0));
}

#[test]
#[cfg_attr(
    debug_assertions,
    should_panic(expected = "for `color`, but the cascade's winner is")
)]
fn winners_that_name_a_declaration_the_cascade_did_not_take_panic_naming_the_property() {
    // `.card` has no declaration 1
    updated(key(1));
}
