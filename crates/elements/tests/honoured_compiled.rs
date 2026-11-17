//! `check_honoured`'s declared-only walk on an element styled from its
//! key's compiled winners (B3a): the longhands a compiled winner
//! declares are the ones checked, so a deferred property it names is
//! refused at that element, and not at a child it does not reach, on
//! the debug and the release verification paths alike.

use crate::support;

use support::refusals;
use vapors_css::property::PropertyId;
use vapors_css::sheet_hash::sheet_hash;
use vapors_dom::{
    BaseWinner, CompiledSheet, DeclarationRef, Dom, KeyWinners, PropertyRef, RuleRef, SheetRef,
    StyleKey, Winners,
};
use vapors_elements::Document;
use vapors_text::FontSystem;

const OWN: &str = "spinners/spinner.wisp";
const SPINNER: &str = ".spinner { animation-name: spin; }";
const DEFERRED: &str = "is not implemented by vapors-elements yet";

/// A key of `div.spinner` whose compiled winners say `animation-name` is
/// won by SPINNER's only declaration, every user-agent rule left to run
/// time.
fn spinner_key() -> &'static StyleKey {
    let name = PropertyId::from_name("animation-name").expect("a longhand");
    let base = vec![BaseWinner {
        property: PropertyRef::Longhand(u16::try_from(name.index()).expect("fits")),
        declaration: DeclarationRef::Rule {
            rule: RuleRef { sheet: 1, rule: 0 },
            declaration: 0,
        },
    }];
    let sheets = vec![
        (
            SheetRef::UserAgent,
            sheet_hash(vapors_css::user_agent::TEXT),
        ),
        (SheetRef::Scope(OWN), sheet_hash(SPINNER)),
    ];
    let winners = Box::leak(Box::new(KeyWinners {
        sheets: sheets.leak(),
        base: base.leak(),
        conditional: user_agent_rules().leak(),
        blocked_by: &[],
        own: Some(1),
    }));
    Box::leak(Box::new(StyleKey {
        site: "spinners/spinner.wisp:1:1",
        scope: Some(OWN),
        tag: "div",
        classes: &["spinner"],
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

#[test]
fn a_deferred_property_a_compiled_winner_declares_is_refused_there_only() {
    for release in [false, true] {
        // Given: a compiled spinner with a plain child
        let mut doc = Document::new(FontSystem::bundle_only());
        if release {
            doc.verify_as_release();
        }
        doc.add_style(CompiledSheet::for_tests(OWN, SPINNER));
        let body = doc.body();
        let spinner = doc.create_keyed_element(spinner_key());
        doc.insert_before(body, spinner, None);
        let child = doc.create_element("span");
        doc.set_attribute(child, "class", "inside");
        doc.insert_before(spinner, child, None);

        // When
        let errors = refusals(&mut doc);

        // Then: the spinner's style came from its compiled winners, and
        // one refusal, at the spinner, for animation-name
        let counts = doc.update_counts();
        assert_eq!(
            counts.winners_compiled, 1,
            "release path {release}: {counts:?}"
        );
        let deferred: Vec<&String> = errors.iter().filter(|e| e.contains(DEFERRED)).collect();
        assert_eq!(deferred.len(), 1, "release path {release}: {errors:?}");
        assert!(
            deferred[0].contains("animation-name") && !deferred[0].contains("inside"),
            "release path {release}: {}",
            deferred[0]
        );
    }
}
