//! `analysis::classify` against the matcher (`support/oracle.rs` holds
//! the model): for each selector, raw and scoped as `scope::scoped`
//! scopes it, every concrete element the key can make is built in a
//! `Tree` as `create_keyed_element` builds it and matched. `Always`
//! must match every one and `Never` none; a `Conditional` verdict must
//! be a function of the inputs its `Depends` names, so a dependency
//! left out shows as two situations equal on the named inputs with
//! different outcomes. The literal attributes are not varied: the key
//! fixes them at run time, as it fixes `class`.

#[path = "support/oracle.rs"]
mod oracle;

use std::collections::BTreeMap;

use oracle::{Situation, judged, named_inputs};
use vapors_css::analysis::Match;
use vapors_css::ast::Rule;

/// Every selector shape the classifier judges, over the key.
const SELECTORS: &[&str] = &[
    "button",
    "div",
    ".card",
    ".open",
    ".busy.open",
    ".other",
    "button.card",
    "button.card.open",
    "[data-wisp=\"s\"]",
    "[data-wisp=\"t\"]",
    "[type=text i]",
    "[type=text]",
    "[style]",
    "[class~=card]",
    "[class~=open]",
    "[class]",
    "#main",
    "[id]",
    "[hidden]",
    ":hover",
    "button:hover",
    ".open:hover",
    ":not(.open)",
    ":not(.other)",
    ":not(.card)",
    ":is(.open, div)",
    ":is(.card, div)",
    ":not(:is(button, div))",
    ":first-child",
    ":last-child",
    ":only-child",
    ":nth-child(2)",
    ":first-of-type",
    ":empty",
    ":root",
    ".panel button",
    ".panel > .card",
    "span + button",
    ":global(.dark) .card",
    ":global(button)",
    ":has(> span)",
    "*",
];

/// Each selector as written and as a component's sheet scopes it
/// (`scope::scoped` changes the rule's selector, not its text), each
/// with a label for messages.
fn forms() -> Vec<(String, Rule)> {
    SELECTORS
        .iter()
        .flat_map(|selector| {
            let css = format!("{selector} {{ color: red }}");
            let raw = vapors_css::parse(&css).expect("parses");
            let scoped = vapors_css::scope::scoped(raw.clone(), "data-wisp", "s");
            [
                (css.clone(), raw.rules[0].clone()),
                (format!("{css} scoped"), scoped.rules[0].clone()),
            ]
        })
        .collect()
}

#[test]
fn always_matches_every_situation_never_matches_none_and_conditional_follows_its_inputs() {
    for (css, rule) in forms() {
        let (verdict, outcomes) = judged(&rule);
        match verdict {
            Match::Always => assert!(
                outcomes.iter().all(|(_, m)| *m),
                "{css}: Always, but some situation does not match"
            ),
            Match::Never => assert!(
                !outcomes.iter().any(|(_, m)| *m),
                "{css}: Never, but some situation matches"
            ),
            Match::Conditional(depends) => {
                let mut seen: BTreeMap<Vec<bool>, (Situation, bool)> = BTreeMap::new();
                for (situation, outcome) in outcomes {
                    let inputs = named_inputs(&depends, situation);
                    let first = *seen.entry(inputs).or_insert((situation, outcome));
                    assert_eq!(
                        first.1, outcome,
                        "{css}: depends on ({depends}) only, but {first:?} and {situation:?} differ"
                    );
                }
            }
        }
    }
}

#[test]
fn the_table_has_each_verdict_and_each_situation_bit_changes_some_outcome() {
    // Given
    let judged: Vec<(Match, Vec<(Situation, bool)>)> =
        forms().iter().map(|(_, rule)| judged(rule)).collect();

    // Then: all three verdicts occur, and every situation bit flips
    // some outcome, so no bit is vacuous
    assert!(judged.iter().any(|(v, _)| *v == Match::Always));
    assert!(judged.iter().any(|(v, _)| *v == Match::Never));
    assert!(
        judged
            .iter()
            .any(|(v, _)| matches!(v, Match::Conditional(_)))
    );
    for bit in 0..7u8 {
        let flips = judged.iter().any(|(_, outcomes)| {
            let by_bits: BTreeMap<(u8, u8), bool> = outcomes
                .iter()
                .map(|(s, m)| ((s.toggles, s.bits), *m))
                .collect();
            by_bits
                .iter()
                .any(|((toggles, bits), m)| by_bits.get(&(*toggles, bits ^ (1 << bit))) != Some(m))
        });
        assert!(
            flips,
            "situation bit {bit} changes no outcome: a vacuous input"
        );
    }
}
