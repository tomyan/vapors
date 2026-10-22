//! Helpers that were in `tests/animation_specs.rs`.
#![allow(dead_code)]

use super::{chain, computed};
use vapors_css::animation::{
    AnimationSpec, TransitionProperty, TransitionSpec, animations, transitions,
};

pub fn transitions_of(decls: &str) -> Vec<TransitionSpec> {
    let (tree, leaf) = chain(&["div.x"]);
    transitions(&computed(&format!(".x {{ {decls} }}"), &tree, leaf))
}

pub fn animations_of(decls: &str) -> Vec<AnimationSpec> {
    let (tree, leaf) = chain(&["div.x"]);
    animations(&computed(&format!(".x {{ {decls} }}"), &tree, leaf))
}

pub fn property(name: &str) -> TransitionProperty {
    TransitionProperty::Property(vapors_css::property::PropertyId::from_name(name).expect(name))
}
