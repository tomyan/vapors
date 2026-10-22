//! Origins, importance, inheritance and computed-value resolution.
//! vapors-specific: sumi had one origin, no !important and no units.

mod support;

use support::{add, chain, computed, css, css_at, env, sheet};
use vapors_css::cascade::Cascade;
use vapors_css::tree::Tree;
use vapors_css::value::ColorScheme;

const RED: &str = "rgb(255 0 0)";
const BLUE: &str = "rgb(0 0 255)";

#[test]
fn important_beats_specificity_and_order() {
    let src = "#main { color: blue; } .a { color: red !important; } .a { color: blue; }";
    assert_eq!(css_at(src, &["span#main.a"], "color"), RED);
}

fn color_with_ua(ua: &str, author: &str) -> String {
    let (tree, leaf) = chain(&["span.a"]);
    let cascade = Cascade::new().user_agent(sheet(ua)).author(sheet(author));
    let style = cascade
        .compute(&tree.node(leaf), None, &env())
        .expect("computes");
    css(&style, "color")
}

#[test]
fn author_rules_beat_user_agent_rules_at_any_specificity() {
    assert_eq!(
        color_with_ua("#x, span.a.b { color: blue; }", ".a { color: red; }"),
        RED
    );
    assert_eq!(color_with_ua("span { color: blue; }", ""), BLUE);
}

#[test]
fn author_important_beats_user_agent_normal_and_loses_to_user_agent_important() {
    assert_eq!(
        color_with_ua("span { color: blue; }", ".a { color: red !important; }"),
        RED
    );
    assert_eq!(
        color_with_ua(
            "span { color: blue !important; }",
            ".a { color: red !important; }"
        ),
        BLUE
    );
}

#[test]
fn inherited_properties_inherit_and_others_take_initial() {
    // Given
    let mut tree = Tree::new("root");
    let root = tree.root();
    let div = add(&mut tree, root, "div.p");
    let span = add(&mut tree, div, "span");
    let src = ".p { color: red; opacity: 0.5; font-size: 20px; }";

    // Then
    let style = computed(src, &tree, span);
    assert_eq!(css(&style, "color"), RED);
    assert_eq!(css(&style, "font-size"), "20px");
    assert_eq!(css(&style, "opacity"), "1");
}

#[test]
fn css_wide_keywords() {
    let mut tree = Tree::new("root");
    let root = tree.root();
    let div = add(&mut tree, root, "div.p");
    let span = add(&mut tree, div, "span.c");
    let src = ".p { color: red; opacity: 0.5; } .c { color: initial; opacity: inherit; width: unset; font-size: unset; }";
    let style = computed(src, &tree, span);
    assert_eq!(css(&style, "color"), "rgb(0 0 0)");
    assert_eq!(css(&style, "opacity"), "0.5");
    assert_eq!(css(&style, "width"), "auto");
    assert_eq!(css(&style, "font-size"), "16px");
}

#[test]
fn relative_units_resolve_against_font_sizes_and_viewport() {
    let mut tree = Tree::new("root");
    let root = tree.root();
    let div = add(&mut tree, root, "div.p");
    let span = add(&mut tree, div, "span.c");
    let src = "root { font-size: 10px; } .p { font-size: 2em; } .c { font-size: 150%; width: 2em; height: 1rem; margin-top: 10vw; margin-left: 10vh; padding-top: 2ch; }";
    let style = computed(src, &tree, span);
    assert_eq!(
        css(&style, "font-size"),
        "30px",
        "150% of the parent's 20px"
    );
    assert_eq!(
        css(&style, "width"),
        "60px",
        "em of the element's own font size"
    );
    assert_eq!(css(&style, "height"), "10px", "rem is the root's font size");
    assert_eq!(css(&style, "margin-top"), "80px");
    assert_eq!(css(&style, "margin-left"), "60px");
    assert_eq!(
        css(&style, "padding-top"),
        "30px",
        "ch is zero_advance_em (0.5) of 30px"
    );
    assert_eq!(style.font_size(), 30.0);
}

#[test]
fn percentages_other_than_font_size_stay_percentages() {
    let (tree, leaf) = chain(&["div.x"]);
    let style = computed(
        ".x { width: 50%; line-height: 150%; font-size: 20px; }",
        &tree,
        leaf,
    );
    assert_eq!(css(&style, "width"), "50%");
    assert_eq!(
        css(&style, "line-height"),
        "30px",
        "line-height % resolves against own font size"
    );
}

#[test]
fn current_color_and_light_dark_resolve_at_compute() {
    let mut tree = Tree::new("root");
    let root = tree.root();
    let div = add(&mut tree, root, "div.p");
    let span = add(&mut tree, div, "span.c");
    let src = ".p { color: red; } .c { border-top-color: currentColor; background-color: light-dark(white, black); color: currentColor; }";
    let cascade = Cascade::new().author(sheet(src));
    let mut e = env();
    e.color_scheme = ColorScheme::Dark;
    let styles = cascade
        .compute_tree(&tree.node(root), &e)
        .expect("computes");
    let (_, style) = styles.iter().find(|(n, _)| n.id() == span).expect("span");
    assert_eq!(css(style, "border-top-color"), RED);
    assert_eq!(css(style, "background-color"), "rgb(0 0 0)");
    assert_eq!(
        css(style, "color"),
        RED,
        "color: currentColor is the parent's colour"
    );
    assert_eq!(style.color().to_rgba8(), [255, 0, 0, 255]);
}

#[test]
fn custom_properties_are_readable_on_the_computed_style() {
    let (tree, leaf) = chain(&["div.x"]);
    let style = computed("html { --accent: cyan; } .x { --gap: 4px; }", &tree, leaf);
    assert_eq!(
        style
            .custom("--accent")
            .map(vapors_css::serialize::component_values)
            .as_deref(),
        Some("cyan")
    );
    assert_eq!(
        style
            .custom("--gap")
            .map(vapors_css::serialize::component_values)
            .as_deref(),
        Some("4px")
    );
    assert_eq!(style.custom("--none"), None);
}

#[test]
fn keyframes_are_reachable_by_name() {
    let cascade = Cascade::new().author(sheet(
        "@keyframes pulse { from { opacity: 0; } to { opacity: 1; } }",
    ));
    assert_eq!(cascade.keyframes("pulse").map(|k| k.stops.len()), Some(2));
    assert!(cascade.keyframes("nope").is_none());
}

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

#[test]
fn unknown_keyframes_error_names_the_declaration_position() {
    let (tree, leaf) = chain(&["div.x"]);
    let cascade = Cascade::new().author(sheet(".x {\n  animation: nope 1s;\n}"));
    let style = cascade
        .compute(&tree.node(leaf), None, &env())
        .expect("computes");
    let spec = vapors_css::animation::animations(&style)
        .into_iter()
        .next()
        .expect("spec");
    let err = cascade.animation(&spec, &style, &env()).unwrap_err();
    assert_eq!((err.position.line, err.position.column), (2, 3), "{err}");
}
