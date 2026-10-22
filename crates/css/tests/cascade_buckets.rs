//! Rule buckets (`src/cascade/buckets.rs`).

mod support;

use support::{chain, sheet};
use vapors_css::cascade::Cascade;

#[test]
fn rules_are_bucketed_by_the_subject_compound() {
    // Given: review item 16; a `span` rule is never a candidate for a div,
    // nor is `.b.a` (keyed on its first class) for an element without .b.
    let (tree, leaf) = chain(&["div.a"]);
    let cascade = Cascade::new().author(sheet("span { color: red; } #x { color: red; } .a { color: red; } div { color: red; } * { color: red; } .b.a { color: red; }"));
    let candidates: Vec<String> = cascade
        .candidates(&tree.node(leaf))
        .map(vapors_css::ast::Rule::selector_text)
        .collect();
    assert_eq!(candidates, [".a", "div", "*"]);
}
