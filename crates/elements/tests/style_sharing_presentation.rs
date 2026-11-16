//! Presentation attributes on keyed elements (S4a PR 1, #376 review
//! H1): a template fixes them, as it fixes `style`, so the build-time
//! analysis sees every one. Setting one on a keyed svg or path is
//! refused naming the site; two keys with different literal values
//! keep their own computed styles; the three names are the cascade's.

use crate::support;

use support::{document, viewport};
use vapors_dom::{Dom, NodeId, StyleKey};
use vapors_elements::Document;

const SCOPE: &str = "vapors-perf/ui/icon.wisp";

const fn key(site: &'static str, attributes: &'static [(&'static str, &'static str)]) -> StyleKey {
    StyleKey {
        site,
        scope: Some(SCOPE),
        tag: "svg",
        classes: &[],
        toggles: &[],
        attributes,
        style: None,
        winners: vapors_dom::Winners::Unanalysed,
    }
}

static THIN: StyleKey = key(
    "crates/perf/ui/icon.wisp:3",
    &[
        ("viewBox", "0 0 16 16"),
        ("width", "16"),
        ("height", "16"),
        ("aria-hidden", "true"),
        ("stroke-width", "2"),
    ],
);
static WIDE: StyleKey = key(
    "crates/perf/ui/icon.wisp:4",
    &[
        ("viewBox", "0 0 16 16"),
        ("width", "16"),
        ("height", "16"),
        ("aria-hidden", "true"),
        ("stroke-width", "3"),
    ],
);

/// A keyed svg of `key`, which sets its literal attributes, holding one
/// stroked path, under the body.
fn icon(doc: &mut Document, key: &'static StyleKey) -> NodeId {
    let svg = doc.create_keyed_element(key);
    let path = doc.create_element("path");
    for (name, value) in [
        ("d", "M2 8h12"),
        ("fill", "none"),
        ("stroke", "currentColor"),
    ] {
        doc.set_attribute(path, name, value);
    }
    doc.append(svg, path);
    let body = doc.body();
    doc.append(body, svg);
    svg
}

fn stroke_width(doc: &Document, node: NodeId) -> String {
    doc.computed_style(node)
        .expect("styled")
        .get_by_name("stroke-width")
        .expect("a longhand")
        .to_css()
}

#[test]
fn keys_with_different_literal_stroke_widths_keep_their_own() {
    // Given
    let mut doc = document();
    doc.add_style(SCOPE, "svg { display: block; }");
    let (thin, wide) = (icon(&mut doc, &THIN), icon(&mut doc, &WIDE));

    // When
    doc.update(viewport(200.0, 100.0)).expect("updates");

    // Then
    assert_eq!(
        [stroke_width(&doc, thin), stroke_width(&doc, wide)],
        ["2px", "3px"]
    );
}

#[test]
fn setting_a_presentation_attribute_on_a_keyed_svg_is_refused_naming_the_site() {
    // Given
    let refusal = THIN.refusal("fill");

    // Then
    assert_eq!(
        refusal.as_deref(),
        Some(
            "`fill` on <svg> at crates/perf/ui/icon.wisp:3 is set from its template; write it there, or set the property under a class toggled with `class:name={bool}`"
        )
    );
}

#[test]
fn the_key_refuses_the_presentation_attributes_the_cascade_reads() {
    assert_eq!(
        vapors_dom::PRESENTATION_ATTRIBUTES,
        vapors_css::cascade::PRESENTATION_ATTRIBUTES
    );
}

#[test]
fn the_key_and_the_cascade_take_presentation_attributes_on_the_same_elements() {
    assert_eq!(vapors_dom::SVG_TAGS, vapors_css::cascade::SVG_TAGS);
}
