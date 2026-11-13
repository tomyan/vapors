//! `analysis::classify` against the matcher: for each selector and key,
//! every concrete element the key can make (each directive on or off,
//! hovered or not, an expression attribute set or absent, at the root
//! or under a parent, first or second child) is built in a `Tree` and
//! matched. `Always` must match every one and `Never` none; a
//! `Conditional` answer is not constrained here (`analysis_classify.rs`
//! pins what it depends on).

use vapors_css::analysis::{KeyFacts, Match, classify};
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
    expression_attributes: &["id"],
};

/// One run-time situation of a key's element: which directives are on,
/// and four facts as bits.
#[derive(Clone, Copy)]
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
    fn second_child(self) -> bool {
        self.bits & 8 != 0
    }
}

fn situations() -> Vec<Situation> {
    let mut out = Vec::new();
    for toggles in 0..4u8 {
        for bits in 0..16u8 {
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
    if situation.under_parent() && situation.second_child() {
        tree.append(parent, "span");
    }
    let element = if situation.under_parent() {
        tree.append(parent, facts.tag)
    } else {
        parent
    };
    fill(&mut tree, element, facts, situation);
    (tree, element)
}

fn fill(tree: &mut Tree, element: NodeId, facts: &KeyFacts<'_>, situation: Situation) {
    if let Some(scope) = facts.scope {
        tree.set_attribute(element, facts.scope_attribute, scope);
    }
    for class in facts.classes {
        tree.add_class(element, class);
    }
    for (index, toggle) in facts.toggles.iter().enumerate() {
        if situation.toggles & (1 << index) != 0 {
            tree.add_class(element, toggle);
        }
    }
    if let Some(style) = facts.style {
        tree.set_attribute(element, "style", style);
    }
    for (name, value) in facts.literal_attributes {
        tree.set_attribute(element, name, value);
    }
    if situation.id_set() {
        tree.set_id(element, "main");
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
    "[class]",
    "#main",
    "[id]",
    "[hidden]",
    ":hover",
    "button:hover",
    ":not(.open)",
    ":not(.other)",
    ":not(.card)",
    ":is(.open, div)",
    ":is(.card, div)",
    ":not(:is(button, div))",
    ":first-child",
    ":last-child",
    ":nth-child(2)",
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

#[test]
fn always_matches_every_situation_and_never_matches_none() {
    for selector in SELECTORS {
        // Given
        let css = format!("{selector} {{ color: red }}");
        let sheet = vapors_css::parse(&css).unwrap_or_else(|e| panic!("{css}: {e}"));
        let rule = &sheet.rules[0];
        let verdict = classify(rule, &KEY);

        // When
        let outcomes: Vec<bool> = situations()
            .into_iter()
            .map(|situation| {
                let (tree, element) = build(&KEY, situation);
                matches(&rule.selector, &tree.node(element))
            })
            .collect();

        // Then
        match verdict {
            Match::Always => assert!(
                outcomes.iter().all(|m| *m),
                "{selector}: Always, but some situation does not match"
            ),
            Match::Never => assert!(
                !outcomes.iter().any(|m| *m),
                "{selector}: Never, but some situation matches"
            ),
            Match::Conditional(_) => {}
        }
    }
}

#[test]
fn the_table_has_each_verdict_so_the_oracle_checks_all_three() {
    let verdicts: Vec<Match> = SELECTORS
        .iter()
        .map(|selector| {
            let sheet = vapors_css::parse(&format!("{selector} {{ }}")).expect("parses");
            classify(&sheet.rules[0], &KEY)
        })
        .collect();
    assert!(verdicts.contains(&Match::Always));
    assert!(verdicts.contains(&Match::Never));
    assert!(verdicts.iter().any(|v| matches!(v, Match::Conditional(_))));
}
