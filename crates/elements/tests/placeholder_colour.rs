//! A placeholder's colour (`docs/placeholder.md`) against the pinned
//! Chromium's, saved in `placeholder_chromium/computed.txt` from
//! `probe.html` under the text page's sheet (regenerate with
//! `scripts/placeholder-chromium.sh`; no browser runs here): the
//! default, an author's `::placeholder { color }` over it, and typed
//! text in the field's own colour.

use crate::support;

use support::text_page::{Page, click, page, type_text};
use vapors_dom::Dom;
use vapors_scene::{Color, Item};

const COMPUTED: &str = include_str!("placeholder_chromium/computed.txt");

/// The probe's classes, as `probe.html` styles them.
const SHEET: &str = ".author { color: #111827; } .styled { color: #111827; } .styled::placeholder { color: #6b7280; }";

/// Chromium's placeholder and field colours for the probe's `class`,
/// with its placeholder opacity.
fn chromium(class: &str) -> (Color, Color, f32) {
    let row = COMPUTED
        .lines()
        .find(|row| row.split(' ').next() == Some(class))
        .unwrap_or_else(|| panic!("no {class} row in computed.txt; regenerate it"));
    let field = |name: &str| row.split(&format!("{name}=")).nth(1).expect(name);
    let rgb = |name: &str| {
        let [r, g, b] = [0, 1, 2].map(|i| {
            field(name)
                .split(' ')
                .nth(i)
                .expect(name)
                .parse()
                .expect(name)
        });
        Color::from_rgba8(r, g, b, 255)
    };
    let opacity = field("opacity")
        .split(' ')
        .next()
        .expect("opacity")
        .parse()
        .expect("a number");
    (rgb("placeholder"), rgb("input"), opacity)
}

/// The text page's empty field as the probe's `class`, with "Search".
fn field(class: &str) -> Page {
    let page = page();
    {
        let mut doc = page.doc.borrow_mut();
        doc.add_stylesheet("probe.css", SHEET).expect("parses");
        doc.set_attribute(page.plain, "class", class);
        doc.set_attribute(page.plain, "placeholder", "Search");
        if class == "number" {
            doc.set_attribute(page.plain, "type", "number");
        }
    }
    page
}

/// The colour of each run of `text` the page paints.
fn colours_of(page: &Page, text: &str) -> Vec<Color> {
    let mut doc = page.doc.borrow_mut();
    let painted = doc
        .update(support::viewport(400.0, 100.0))
        .expect("updates");
    let scene = painted.scene().to_scene();
    let runs = scene.items().iter().filter_map(|item| match item {
        Item::Text { text: id, .. } => Some(painted.text(*id)),
        _ => None,
    });
    runs.filter(|layout| layout.text() == text)
        .map(|layout| layout.runs()[0].color)
        .collect()
}

#[test]
fn each_placeholder_is_painted_in_the_pinned_chromiums_colour() {
    for class in ["plain", "author", "styled", "number"] {
        // Given
        let (expected, _, opacity) = chromium(class);

        // When
        let painted = colours_of(&field(class), "Search");

        // Then: opaque, as Chromium's, and its colour
        assert_eq!(
            opacity, 1.0,
            "{class}: vapors paints no placeholder opacity"
        );
        assert_eq!(painted, [expected], "{class}");
    }
}

#[test]
fn typed_text_keeps_the_fields_own_colour_beside_a_placeholder_colour() {
    // Given: the styled field, focused
    let page = field("styled");
    let (_, own, _) = chromium("styled");
    click(&page, [80.0, 36.0]);

    // When
    type_text(&page, "ada");

    // Then
    assert_eq!(colours_of(&page, "ada"), [own]);
    assert_eq!(colours_of(&page, "Search"), []);
}

#[test]
fn a_class_set_at_run_time_recolours_the_placeholder() {
    // Given: a plain field, laid out
    let page = field("plain");
    let (plain, _, _) = chromium("plain");
    let (styled, _, _) = chromium("styled");
    assert_eq!(colours_of(&page, "Search"), [plain]);

    // When
    page.doc
        .borrow_mut()
        .set_attribute(page.plain, "class", "styled");

    // Then
    assert_eq!(colours_of(&page, "Search"), [styled]);
}
