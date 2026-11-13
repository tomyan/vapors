//! One compound against a key's fixed parts. Each piece is judged with
//! the matcher's own test where a value is fixed (the attribute operator
//! through `selector::matching::attribute_matches`), and is conditional
//! where the key leaves it to run time.

use super::{Depends, KeyFacts, Match};
use crate::selector::{AttributeSelector, Compound, PseudoClass, matching::attribute_matches};

/// Whether a key's elements satisfy `compound`, without its context.
pub(super) fn judge(compound: &Compound, facts: &KeyFacts<'_>) -> Match {
    let mut result = tag(compound.tag.as_deref(), facts);
    if let Some(id) = &compound.id {
        result = result.and(attribute_equals("id", id, facts));
    }
    for class in &compound.classes {
        result = result.and(class_present(class, facts));
    }
    for attribute in &compound.attributes {
        result = result.and(attribute_selector(attribute, facts));
    }
    for pseudo in &compound.pseudo_classes {
        result = result.and(pseudo_class(pseudo, facts));
    }
    result
}

fn tag(tag: Option<&str>, facts: &KeyFacts<'_>) -> Match {
    match tag {
        Some(tag) if !tag.eq_ignore_ascii_case(facts.tag) => Match::Never,
        _ => Match::Always,
    }
}

/// `.class`: the matcher's `has_class` reads the class list, which a
/// keyed element changes only through its directives.
fn class_present(class: &str, facts: &KeyFacts<'_>) -> Match {
    if facts.classes.contains(&class) {
        return Match::Always;
    }
    if facts.toggles.contains(&class) {
        return Match::depends(Depends {
            toggles: [class.to_owned()].into(),
            ..Depends::default()
        });
    }
    Match::Never
}

/// What the key says about an attribute's value.
enum Fact<'a> {
    /// Set to this value, or absent, on every element.
    Fixed(Option<&'a str>),
    /// Decided at run time, by these inputs.
    Runtime(Depends),
}

fn attribute_fact<'a>(name: &str, facts: &KeyFacts<'a>) -> Fact<'a> {
    if name == facts.scope_attribute {
        return Fact::Fixed(facts.scope);
    }
    if name == "class" {
        return class_attribute(facts);
    }
    if name == "style" {
        return Fact::Fixed(facts.style);
    }
    if let Some((_, value)) = facts.literal_attributes.iter().find(|(n, _)| *n == name) {
        return Fact::Fixed(Some(value));
    }
    // an expression attribute, or one the template does not set, which
    // `set_attribute` may still set at run time
    Fact::Runtime(Depends {
        attributes: [name.to_owned()].into(),
        ..Depends::default()
    })
}

/// The `class` attribute as text (`[class~=a]`): absent when the key
/// sets no class; otherwise left to the matcher at run time rather than
/// reconstructing the text the key writes, which reads the directives.
fn class_attribute<'a>(facts: &KeyFacts<'a>) -> Fact<'a> {
    if facts.classes.is_empty() && facts.toggles.is_empty() {
        return Fact::Fixed(None);
    }
    Fact::Runtime(Depends {
        toggles: facts.toggles.iter().map(|t| (*t).to_owned()).collect(),
        attributes: ["class".to_owned()].into(),
        ..Depends::default()
    })
}

/// `#id`: equal to a fixed value, or run time.
fn attribute_equals(name: &str, want: &str, facts: &KeyFacts<'_>) -> Match {
    match attribute_fact(name, facts) {
        Fact::Fixed(Some(value)) if value == want => Match::Always,
        Fact::Fixed(_) => Match::Never,
        Fact::Runtime(depends) => Match::depends(depends),
    }
}

fn attribute_selector(attribute: &AttributeSelector, facts: &KeyFacts<'_>) -> Match {
    match attribute_fact(&attribute.name, facts) {
        Fact::Fixed(Some(value)) if attribute_matches(attribute, value) => Match::Always,
        Fact::Fixed(_) => Match::Never,
        Fact::Runtime(depends) => Match::depends(depends),
    }
}

/// Every pseudo-class the matcher knows, each placed: a state flag or
/// structure is run time; `:root` is the mount site's; the lists
/// combine their arguments as the matcher does.
fn pseudo_class(pseudo: &PseudoClass, facts: &KeyFacts<'_>) -> Match {
    if let Some(flag) = crate::selector::state_flag(pseudo) {
        return Match::depends(Depends {
            states: flag,
            ..Depends::default()
        });
    }
    match pseudo {
        PseudoClass::Root => Match::depends(Depends {
            ancestors: true,
            ..Depends::default()
        }),
        PseudoClass::Not(list) => list
            .iter()
            .map(|selector| super::selector(selector, facts))
            .fold(Match::Never, Match::or)
            .not(),
        PseudoClass::Is(list) | PseudoClass::Where(list) | PseudoClass::Global(list) => list
            .iter()
            .map(|selector| super::selector(selector, facts))
            .fold(Match::Never, Match::or),
        PseudoClass::Empty
        | PseudoClass::Has(_)
        | PseudoClass::FirstChild
        | PseudoClass::LastChild
        | PseudoClass::OnlyChild
        | PseudoClass::FirstOfType
        | PseudoClass::LastOfType
        | PseudoClass::OnlyOfType
        | PseudoClass::NthChild(_)
        | PseudoClass::NthLastChild(_)
        | PseudoClass::NthOfType(_)
        | PseudoClass::NthLastOfType(_) => Match::depends(Depends {
            structure: true,
            ..Depends::default()
        }),
        PseudoClass::Hover
        | PseudoClass::Focus
        | PseudoClass::FocusVisible
        | PseudoClass::FocusWithin
        | PseudoClass::Active
        | PseudoClass::Checked
        | PseudoClass::Disabled
        | PseudoClass::Enabled
        | PseudoClass::Invalid => unreachable!("state_flag places every state pseudo-class"),
    }
}
