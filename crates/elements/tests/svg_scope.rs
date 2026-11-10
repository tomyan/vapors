//! An `svg` and `path` in a component with a `<style>` carry the
//! compiler's scoping attribute (docs/icons.md, The subset): the icon
//! subset accepts that attribute and no other `data-*`, and a scoped
//! rule reaches the icon.

mod support;

use support::svg_page::{assert_accepted, assert_refused, icon};
use support::viewport;
use vapors_dom::{Dom, NodeId, SCOPE_ATTRIBUTE};
use vapors_elements::Document;
use vapors_scene::{Color, Item};

/// Shaped as the compiler emits it: the crate name, a slash, the file
/// (crates/compiler/src/codegen/shell.rs).
const SCOPE: &str = "touche-bar/src/top_bar.wisp";

/// An icon page whose `svg` and its `path` are tagged with `SCOPE`, as
/// a component's `mount` tags every element.
fn scoped_icon(css: &str) -> (Document, NodeId) {
    let (mut doc, svg) = icon(css, &[], &[]);
    doc.set_attribute(svg, SCOPE_ATTRIBUTE, SCOPE);
    let path = doc.children(svg)[0];
    doc.set_attribute(path, SCOPE_ATTRIBUTE, SCOPE);
    (doc, svg)
}

#[test]
fn an_icon_tagged_with_the_scope_attribute_is_accepted() {
    // Given
    let (mut doc, _) = scoped_icon("");

    // When / Then
    assert_accepted(&mut doc);
}

#[test]
fn a_scoped_color_rule_reaches_the_icon() {
    // Given: a rule in the component's scope, none in the global sheet
    let (mut doc, _) = scoped_icon("");
    doc.add_style(SCOPE, ".icon { color: rgb(255, 0, 0); }");

    // When
    let painted = doc.update(viewport(100.0, 100.0)).expect("updates");

    // Then
    let colours: Vec<Color> = painted
        .scene()
        .items()
        .iter()
        .filter_map(|item| match item {
            Item::Icon { colour, .. } => Some(*colour),
            _ => None,
        })
        .collect();
    assert_eq!(colours, [Color::from_rgba8(255, 0, 0, 255)]);
}

#[test]
fn another_data_attribute_on_an_svg_or_path_is_still_refused() {
    // Given
    let (mut on_svg, svg) = scoped_icon("");
    on_svg.set_attribute(svg, "data-other", "x");
    let (mut on_path, svg) = scoped_icon("");
    let path = on_path.children(svg)[0];
    on_path.set_attribute(path, "data-other", "x");

    // When / Then
    assert_refused(
        &mut on_svg,
        &["<svg", "data-other", "not in the icon subset"],
    );
    assert_refused(
        &mut on_path,
        &["<path", "data-other", "not in the icon subset"],
    );
}
