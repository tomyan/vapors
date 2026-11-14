//! The model for `analysis_winners_oracle.rs`: a page's sheets in a
//! `Cascade` in a given order, the key's element in each situation,
//! and what the cascade may report for each winner.

#![allow(dead_code)] // each test binary uses a subset

use super::Page;
use vapors_css::analysis::Winner;
use vapors_css::cascade::{Cascade, ComputedStyle, Environment, INLINE_SHEET};
use vapors_css::property::PropertyId;
use vapors_css::tree::{NodeId, States, Tree};
use vapors_css::{Position, ast::Stylesheet};

/// The cascade with the page's author sheets in `order` (indices into
/// the page's sheets after the user-agent sheet), and the cascade index
/// of each page sheet.
pub fn cascade(page: &Page, order: &[usize]) -> (Cascade, Vec<usize>) {
    let mut cascade = Cascade::new().user_agent(page.sheets[0].clone());
    let mut index = vec![usize::MAX; page.sheets.len()];
    index[0] = 0;
    for (position, sheet) in order.iter().enumerate() {
        cascade = cascade.author(page.sheets[*sheet].clone());
        index[*sheet] = position + 1;
    }
    (cascade, index)
}

/// Both orders of the page's author sheets.
pub fn orders(page: &Page) -> Vec<Vec<usize>> {
    let authors: Vec<usize> = (1..page.sheets.len()).collect();
    let mut reversed = authors.clone();
    reversed.reverse();
    vec![authors, reversed]
}

/// The key's element in a tree, `toggles` on and hovered or not, under
/// `parent_style` (a parent `div` with that literal style) or alone.
pub fn element(
    page: &Page,
    toggles: u8,
    hovered: bool,
    parent_style: Option<&str>,
) -> (Tree, NodeId) {
    let mut tree = Tree::new("div");
    let root = tree.root();
    let node = match parent_style {
        Some(style) => {
            tree.set_attribute(root, "style", style);
            tree.append(root, "div")
        }
        None => root,
    };
    tree.set_attribute(node, "data-wisp", "s");
    let mut classes = vec!["card"];
    if toggles & 1 != 0 {
        classes.push("open");
    }
    for class in &classes {
        tree.add_class(node, class);
    }
    tree.set_attribute(node, "class", &classes.join(" "));
    if let Some(style) = page.style {
        tree.set_attribute(node, "style", style);
    }
    if hovered {
        tree.set_states(node, States::HOVER);
    }
    (tree, node)
}

pub fn computed(
    cascade: &Cascade,
    page: &Page,
    toggles: u8,
    hovered: bool,
    parent: Option<&str>,
) -> ComputedStyle {
    let (tree, node) = element(page, toggles, hovered, parent);
    let env = Environment::default();
    let parent_style = parent.map(|_| {
        cascade
            .compute(&tree.node(tree.root()), None, &env)
            .expect("the parent computes")
    });
    cascade
        .compute(&tree.node(node), parent_style.as_ref(), &env)
        .expect("computes")
}

/// Every (cascade sheet, position) a rule declares `id` at.
pub fn declarations_of(sheet: &Stylesheet, rule: usize, id: PropertyId) -> Vec<Position> {
    sheet.rules[rule]
        .declarations
        .iter()
        .filter(|decl| decl.longhands.iter().any(|longhand| longhand.id == id))
        .map(|decl| decl.position)
        .collect()
}

/// What the cascade may report for `winner`: (sheet index in the
/// cascade, position) pairs, or `None` for no declaration.
pub fn allowed(
    winner: &Winner,
    page: &Page,
    index: &[usize],
    id: PropertyId,
) -> Vec<Option<(usize, Position)>> {
    let inline = page
        .style
        .map(|style| vapors_css::parse_inline(style).expect("inline"))
        .unwrap_or_default();
    let at = |sheet: usize, rule: usize, declaration: usize| {
        if sheet == INLINE_SHEET {
            return (INLINE_SHEET, inline[declaration].position);
        }
        (
            index[sheet],
            page.sheets[sheet].rules[rule].declarations[declaration].position,
        )
    };
    match winner {
        Winner::Constant { place, .. } => {
            vec![Some(at(place.sheet, place.rule, place.declaration))]
        }
        Winner::Undeclared { .. } => vec![None],
        Winner::Variable { base, overriders } => {
            let mut out: Vec<Option<(usize, Position)>> = match base {
                Some(place) => vec![Some(at(place.sheet, place.rule, place.declaration))],
                None => vec![None],
            };
            for (sheet, rule) in overriders {
                for position in declarations_of(&page.sheets[*sheet], *rule, id) {
                    out.push(Some((index[*sheet], position)));
                }
            }
            out
        }
    }
}
