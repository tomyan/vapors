//! The oracle's model for `analysis_oracle.rs`: the key, every
//! run-time situation of its element built as `create_keyed_element`
//! builds it, the selector shapes raw and scoped, and the inputs a
//! `Depends` names.

#![allow(dead_code)] // each test binary uses a subset

use vapors_css::analysis::{Depends, KeyFacts, Match, classify};
use vapors_css::selector::matches;
use vapors_css::tree::{NodeId, States, Tree};

pub const KEY: KeyFacts<'static> = KeyFacts {
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
pub struct Situation {
    pub toggles: u8,
    pub bits: u8,
}

impl Situation {
    pub fn hovered(self) -> bool {
        self.bits & 1 != 0
    }
    pub fn id_set(self) -> bool {
        self.bits & 2 != 0
    }
    pub fn under_parent(self) -> bool {
        self.bits & 4 != 0
    }
    /// A `span` before it; only under a parent.
    pub fn second_child(self) -> bool {
        self.bits & 8 != 0 && self.under_parent()
    }
    pub fn has_child(self) -> bool {
        self.bits & 16 != 0
    }
    /// An `em` after it; only under a parent.
    pub fn has_next_sibling(self) -> bool {
        self.bits & 32 != 0 && self.under_parent()
    }
    pub fn hidden_set(self) -> bool {
        self.bits & 64 != 0
    }
}

pub fn situations() -> Vec<Situation> {
    let mut out = Vec::new();
    for toggles in 0..4u8 {
        for bits in 0..128u8 {
            out.push(Situation { toggles, bits });
        }
    }
    out
}

/// A tree holding the key's element in `situation`; the element's id.
pub fn build(facts: &KeyFacts<'_>, situation: Situation) -> (Tree, NodeId) {
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
pub fn fill(tree: &mut Tree, element: NodeId, facts: &KeyFacts<'_>, situation: Situation) {
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

/// The situation's values of the inputs `depends` names.
pub fn named_inputs(depends: &Depends, situation: Situation) -> Vec<bool> {
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
pub fn judged(css: &str) -> (Match, Vec<(Situation, bool)>) {
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
