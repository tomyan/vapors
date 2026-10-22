//! Shared helpers for the integration tests.
#![allow(dead_code)]

use vapors_css::ast::{Rule, Stylesheet};
use vapors_css::parse;

/// Parse or panic with the refusal text.
pub fn sheet(src: &str) -> Stylesheet {
    match parse(src) {
        Ok(s) => s,
        Err(e) => panic!("parse refused: {e}\n{src}"),
    }
}

/// The refusal for a stylesheet that must not parse.
pub fn refusal(src: &str) -> vapors_css::Error {
    match parse(src) {
        Ok(_) => panic!("expected a refusal for:\n{src}"),
        Err(e) => e,
    }
}

/// The serialized value of a declaration by name, or "" when absent.
pub fn prop(rule: &Rule, name: &str) -> String {
    rule.declarations
        .iter()
        .find(|d| d.name == name)
        .map(vapors_css::ast::Declaration::value_text)
        .unwrap_or_default()
}

use vapors_css::selector::{Selector, matches, parse_selector_list};
use vapors_css::tree::{NodeId, Tree};

/// Parse one selector or panic with the refusal.
pub fn selector(text: &str) -> Selector {
    let mut list = match parse_selector_list(text) {
        Ok(l) => l,
        Err(e) => panic!("selector {text:?} refused: {e}"),
    };
    assert_eq!(list.len(), 1, "{text:?} is a list");
    list.remove(0)
}

/// Add a child described as `tag.class#id[attr=value]`; a tag of `*` is
/// `div`. Only the shapes the tests use.
pub fn add(tree: &mut Tree, parent: NodeId, desc: &str) -> NodeId {
    let (tag, rest) = split_tag(desc);
    let id = tree.append(parent, tag);
    let mut rest = rest;
    while !rest.is_empty() {
        let kind = rest.as_bytes()[0];
        let end = rest[1..]
            .find(['.', '#', '['])
            .map_or(rest.len(), |i| i + 1);
        let (item, tail) = rest.split_at(end);
        match kind {
            b'.' => tree.add_class(id, &item[1..]),
            b'#' => tree.set_id(id, &item[1..]),
            b'[' => {
                let body = item.trim_start_matches('[').trim_end_matches(']');
                let (name, value) = body.split_once('=').unwrap_or((body, ""));
                tree.set_attribute(id, name, value);
            }
            _ => unreachable!(),
        }
        rest = tail;
    }
    id
}

fn split_tag(desc: &str) -> (&str, &str) {
    let end = desc.find(['.', '#', '[']).unwrap_or(desc.len());
    let tag = if end == 0 { "div" } else { &desc[..end] };
    (tag, &desc[end..])
}

/// A root-to-leaf chain of elements; returns the tree and the leaf.
pub fn chain(descs: &[&str]) -> (Tree, NodeId) {
    let mut tree = Tree::new("html");
    let mut node = tree.root();
    for d in descs {
        node = add(&mut tree, node, d);
    }
    (tree, node)
}

/// Whether `selector_text` matches the leaf of `chain`.
pub fn matches_chain(selector_text: &str, descs: &[&str]) -> bool {
    let (tree, leaf) = chain(descs);
    matches(&selector(selector_text), &tree.node(leaf))
}

/// A parent with `children` described like [`add`]; returns the tree and
/// the children in order.
pub fn siblings(children: &[&str]) -> (Tree, Vec<NodeId>) {
    let mut tree = Tree::new("root");
    let parent = tree.root();
    let ids = children.iter().map(|d| add(&mut tree, parent, d)).collect();
    (tree, ids)
}
