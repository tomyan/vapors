//! A presentation attribute set at run time is part of the share key
//! (S4a PR 1): two keyed svgs of one key whose `stroke-width` attributes
//! differ do not share a computed style.

mod support;

use support::{document, viewport};
use vapors_dom::{Dom, StyleKey};

const SCOPE: &str = "vapors-perf/ui/icon.wisp";

static ICON: StyleKey = StyleKey {
    site: "crates/perf/ui/icon.wisp:3",
    scope: Some(SCOPE),
    tag: "svg",
    classes: &[],
    toggles: &[],
    attributes: &[],
    style: None,
};

#[test]
fn keyed_svgs_with_different_stroke_width_attributes_keep_their_own() {
    // Given: two svgs of one key, stroke-width set apart at run time
    let mut doc = document();
    doc.add_style(SCOPE, "svg { display: block; }");
    let body = doc.body();
    let mut svgs = Vec::new();
    for width in ["2", "3"] {
        let svg = doc.create_keyed_element(&ICON);
        let size = [("viewBox", "0 0 16 16"), ("width", "16"), ("height", "16")];
        for (name, value) in size
            .into_iter()
            .chain([("aria-hidden", "true"), ("stroke-width", width)])
        {
            doc.set_attribute(svg, name, value);
        }
        let path = doc.create_element("path");
        for (name, value) in [
            ("d", "M2 8h12"),
            ("fill", "none"),
            ("stroke", "currentColor"),
        ] {
            doc.set_attribute(path, name, value);
        }
        doc.append(svg, path);
        doc.append(body, svg);
        svgs.push(svg);
    }

    // When
    doc.update(viewport(200.0, 100.0)).expect("updates");

    // Then
    let width = |node| {
        doc.computed_style(node)
            .expect("styled")
            .get_by_name("stroke-width")
            .expect("a longhand")
            .to_css()
    };
    assert_eq!([width(svgs[0]), width(svgs[1])], ["2", "3"]);
}
