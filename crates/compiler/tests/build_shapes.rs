//! One-scene S5a, part 1: the build parses, refuses, classifies and
//! deduplicates each crate's literal shapes into its `SHAPES` table.

use crate::common_build as build;

use build::{dirs, write};
use vapors_compiler::build::compile_dir_into;

const TRIANGLE: &str = "M0 0L4 0L2 3Z";
/// An open path that only a stroke can draw.
const OPEN: &str = "<path d=\"M2 4h4\" stroke=\"red\"/>";

/// A component of one svg (with `attrs`) holding `shapes`, on line 2.
fn page(attrs: &str, shapes: &str) -> String {
    format!("<svg viewBox=\"0 0 8 8\" width=\"8\" height=\"8\" {attrs}>\n{shapes}\n</svg>")
}

/// `shapes` in a component whose script has the signals they bind.
fn bound(shapes: &str) -> String {
    let script = "let d: Signal<String> = Signal::new(String::new());\nlet on: Signal<bool> = Signal::new(true);\nlet items: Signal<Vec<(u32, String)>> = Signal::new(Vec::new());";
    format!("<script>\n{script}\n</script>\n{}", page("", shapes))
}

/// The `shapes.rs` the build of `files` wrote (empty for none), or its refusals.
fn built(name: &str, files: &[(&str, String)]) -> Result<String, Vec<String>> {
    let (src, out) = dirs(name);
    for (file, text) in files {
        write(&src, file, text);
    }
    compile_dir_into(&src, &out)
        .map_err(|errs| errs.iter().map(ToString::to_string).collect::<Vec<_>>())?;
    Ok(std::fs::read_to_string(out.join("shapes.rs")).unwrap_or_default())
}

#[test]
fn a_bad_d_and_a_fill_with_no_area_fail_the_build_naming_the_file_and_line() {
    // Given: a bad path in one file, a filled line with no area in another
    let files = [
        ("bad.wisp", page("", "<path d=\"M0 0 X\"/>")),
        ("flat.wisp", page("", "<path d=\"M0 0L4 0Z\"/>")),
    ];

    // When
    let errors = built("refused", &files).expect_err("no").join("\n");

    // Then
    let wants = [
        "bad.wisp:2:1: ",
        "<path> d",
        "flat.wisp:2:1: ",
        "has no area",
    ];
    assert!(wants.iter().all(|want| errors.contains(want)), "{errors}");
}

#[test]
fn the_same_path_in_two_components_is_one_entry_at_its_first_site() {
    // Given: a triangle in both, the first filled on its own over its
    // svg's fill of none, and a bound path in the second
    let second = format!("<path d=\"{TRIANGLE}\"/><path d={{d}}/>");
    let red = format!("<path d=\"{TRIANGLE}\" fill=\"red\"/>");
    let files = [
        ("a.wisp", page("fill=\"none\"", &red)),
        ("b.wisp", bound(&second)),
    ];

    // When
    let table = built("shapes_dedup", &files).expect("builds");

    // Then
    assert_eq!(table.matches("site: ").count(), 1, "{table}");
    assert!(table.contains("\"src/a.wisp:2\""), "{table}");
}

#[test]
fn a_fill_that_is_off_or_unresolvable_gets_no_entry_and_no_refusal() {
    // Given: stroke-only open paths whose fill is off by a literal, by
    // the svg's fill or by CSS
    let css = format!("<style>path {{ fill: none; }}</style>\n{}", page("", OPEN));
    let files = [
        ("own.wisp", page("", "<path d=\"M2 4h4\" fill=\"NONE\"/>")),
        ("svg.wisp", page("fill=\"none\"", OPEN)),
        ("css.wisp", css),
    ];

    // When: the build accepts them, as the runtime does
    let table = built("shapes_fill_off", &files).expect("builds");

    // Then
    assert_eq!(table, "");
}

#[test]
fn literal_shapes_get_entries_whatever_their_paint_or_place_and_bound_ones_none() {
    // Given: a rect, a circle, and a path under each of an if and a for,
    // all literal; and a rect and a circle with a bound geometry attribute
    let literal = format!(
        "<rect x=\"1\" y=\"1\" width=\"4\" height=\"3\" stroke=\"red\"/>\n<circle cx=\"4\" cy=\"4\" r=\"2\" stroke-width=\"2\"/>\n{{if on.get()}}<path d=\"{TRIANGLE}\"/>{{/if}}{{for (i, x) in items.get() key=x.0}}<path d=\"M1 6H5L3 4Z\"/>{{/for}}"
    );
    let unbound =
        "<rect x={d} y=\"1\" width=\"4\" height=\"3\"/>\n<circle cx=\"4\" cy=\"4\" r={d}/>";

    // When
    let table = built("shapes_basic", &[("l.wisp", bound(&literal))]).expect("builds");
    let none = built("shapes_bound", &[("b.wisp", bound(unbound))]).expect("builds");

    // Then
    assert!(
        table.contains("Geometry::Rect {") && table.contains("Geometry::Ellipse {"),
        "{table}"
    );
    assert_eq!(table.matches("site: ").count(), 4, "{table}");
    assert_eq!(none, "");
}
