//! `analysis::classify` against the matcher: for each selector, raw and
//! scoped as `scope::scoped` scopes it, and the key, every concrete
//! element the key can make is built in a `Tree` as
//! `create_keyed_element` builds it (`class` and `id` as attributes the
//! runtime reads them from, beside the tree's own lists) and matched:
//! each directive on or off, hovered, `id` set, `hidden` set, under a
//! parent, second child, with a child, with a following sibling.
//! `Always` must match every one and `Never` none; a `Conditional`
//! verdict must be a function of the inputs its `Depends` names, so a
//! dependency left out shows as two situations equal on the named
//! inputs with different outcomes. The literal attributes are not
//! varied: the key fixes them at run time, as it fixes `class`.

use std::collections::BTreeMap;

use vapors_css::analysis::{Depends, KeyFacts, Match, classify};
use vapors_css::selector::matches;
use vapors_css::tree::{NodeId, States, Tree};

const KEY: KeyFacts<'static> = KeyFacts {
    tag: "button",
    scope_attribute: "data-wisp",
    scope: Some("s"),
    classes: &["card"],
    toggles: &["open", "busy"],
    style: Some("color: red"),
    literal_attributes: &[("type", "Text")],
};

/// One run-time situation of a key's element: which directives are on,
/// and seven facts as bits.
#[derive(Clone, Copy, Debug)]
struct Situation {
    toggles: u8,
    bits: u8,
}

impl Situation {
    fn hovered(self) -> bool {
        self.bits & 1 != 0
    }
    fn id_set(self) -> bool {
        self.bits & 2 != 0
    }
    fn under_parent(self) -> bool {
        self.bits & 4 != 0
    }
    /// A `span` before it; only under a parent.
    fn second_child(self) -> bool {
        self.bits & 8 != 0 && self.under_parent()
    }
    fn has_child(self) -> bool {
        self.bits & 16 != 0
    }
    /// An `em` after it; only under a parent.
    fn has_next_sibling(self) -> bool {
        self.bits & 32 != 0 && self.under_parent()
    }
    fn hidden_set(self) -> bool {
        self.bits & 64 != 0
    }
}

fn situations() -> Vec<Situation> {
    let mut out = Vec::new();
    for toggles in 0..4u8 {
        for bits in 0..128u8 {
            out.push(Situation { toggles, bits });
        }
    }
    out
}

/// A tree holding the key's element in `situation`; the element's id.
fn build(facts: &KeyFacts<'_>, situation: Situation) -> (Tree, NodeId) {
    let mut tree = Tree::new(if situation.under_parent() {
        "div"
    } else {
        facts.tag
    });
    let parent = tree.root();
    if situation.under_parent() {
        tree.add_class(parent, "panel");
    }
    if situation.second_child() {
        tree.append(parent, "span");
    }
    let element = if situation.under_parent() {
        tree.append(parent, facts.tag)
    } else {
        parent
    };
    fill(&mut tree, element, facts, situation);
    if situation.has_child() {
        tree.append(element, "span");
    }
    if situation.has_next_sibling() {
        tree.append(parent, "em");
    }
    (tree, element)
}

/// The element as `create_keyed_element` makes it, then the situation's
/// run-time inputs.
fn fill(tree: &mut Tree, element: NodeId, facts: &KeyFacts<'_>, situation: Situation) {
    if let Some(scope) = facts.scope {
        tree.set_attribute(element, facts.scope_attribute, scope);
    }
    let mut class_names: Vec<&str> = facts.classes.to_vec();
    for (index, toggle) in facts.toggles.iter().enumerate() {
        if situation.toggles & (1 << index) != 0 {
            class_names.push(toggle);
        }
    }
    for class in &class_names {
        tree.add_class(element, class);
    }
    tree.set_attribute(element, "class", &class_names.join(" "));
    if let Some(style) = facts.style {
        tree.set_attribute(element, "style", style);
    }
    for (name, value) in facts.literal_attributes {
        tree.set_attribute(element, name, value);
    }
    if situation.id_set() {
        tree.set_id(element, "main");
        tree.set_attribute(element, "id", "main");
    }
    if situation.hidden_set() {
        tree.set_attribute(element, "hidden", "");
    }
    if situation.hovered() {
        tree.set_states(element, States::HOVER);
    }
}

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

/// The situation's values of the inputs `depends` names.
fn named_inputs(depends: &Depends, situation: Situation) -> Vec<bool> {
    let mut inputs = Vec::new();
    for (index, toggle) in KEY.toggles.iter().enumerate() {
        if depends.toggles.contains(*toggle) || depends.attributes.contains("class") {
            inputs.push(situation.toggles & (1 << index) != 0);
        }
    }
    if depends.states != States::NONE {
        inputs.push(situation.hovered());
    }
    if depends.attributes.contains("id") {
        inputs.push(situation.id_set());
    }
    if depends.attributes.contains("hidden") {
        inputs.push(situation.hidden_set());
    }
    if depends.ancestors {
        inputs.push(situation.under_parent());
    }
    if depends.structure {
        inputs.extend([
            situation.under_parent(),
            situation.second_child(),
            situation.has_child(),
            situation.has_next_sibling(),
        ]);
    }
    inputs
}

/// The rule's verdict and each situation's outcome.
fn judged(css: &str) -> (Match, Vec<(Situation, bool)>) {
    let sheet = vapors_css::parse(css).unwrap_or_else(|e| panic!("{css}: {e}"));
    let rule = &sheet.rules[0];
    let outcomes = situations()
        .into_iter()
        .map(|situation| {
            let (tree, element) = build(&KEY, situation);
            (situation, matches(&rule.selector, &tree.node(element)))
        })
        .collect();
    (classify(rule, &KEY), outcomes)
}

/// Each selector as written and as a component's sheet scopes it.
fn forms() -> Vec<String> {
    SELECTORS
        .iter()
        .flat_map(|selector| {
            let raw = format!("{selector} {{ color: red }}");
            let sheet = vapors_css::parse(&raw).expect("parses");
            let scoped = vapors_css::scope::scoped(sheet, "data-wisp", "s").to_css();
            [raw, scoped]
        })
        .collect()
}

#[test]
fn always_matches_every_situation_never_matches_none_and_conditional_follows_its_inputs() {
    for css in forms() {
        let (verdict, outcomes) = judged(&css);
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
        forms().iter().map(|css| judged(css)).collect();

    // Then: all three verdicts occur, and every situation bit flips
    // some outcome, so no bit is vacuous
    assert!(judged.iter().any(|(v, _)| *v == Match::Always));
    assert!(judged.iter().any(|(v, _)| *v == Match::Never));
    assert!(judged.iter().any(|(v, _)| matches!(v, Match::Conditional(_))));
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
        assert!(flips, "situation bit {bit} changes no outcome: a vacuous input");
    }
}
