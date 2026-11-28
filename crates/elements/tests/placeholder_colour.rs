//! A placeholder's colour (`docs/placeholder.md`) against the pinned
//! Chromium's, saved in `placeholder_chromium/computed.txt` from
//! `probe.html` under the text page's sheet (regenerate with
//! `scripts/placeholder-chromium.sh`; no browser runs here): the
//! default, an author's `::placeholder { color }` over it (by
//! specificity, and as `currentcolor`), and typed text in the field's own
//! colour.

use crate::support;

use support::text_page::{Page, click, page, type_text};
use vapors_dom::Dom;
use vapors_scene::{Color, Item};

const COMPUTED: &str = include_str!("placeholder_chromium/computed.txt");
const TOOLS: &str = include_str!("../../../.config/tools.toml");

/// The probe's classes, as `probe.html` styles them.
const SHEET: &str = ".author { color: #111827; } .styled { color: #111827; } .styled::placeholder { color: #6b7280; } input.specific::placeholder { color: #0000ff; } .specific::placeholder { color: #ff0000; } .current { color: #008000; } .current::placeholder { color: currentcolor; }";

/// Every row of `computed.txt`, one per probe input.
const CLASSES: [&str; 6] = ["plain", "author", "styled", "number", "specific", "current"];

/// What Chromium computed for one probe input.
struct Row {
    placeholder: Color,
    fill: Color,
    field: Color,
    opacity: f32,
}

/// Chromium's row for the probe's `class`.
fn chromium(class: &str) -> Row {
    let row = COMPUTED
        .lines()
        .find(|row| row.split(' ').next() == Some(class))
        .unwrap_or_else(|| panic!("no {class} row in computed.txt; regenerate it"));
    let words = |name: &str| {
        row.split(&format!("{name}="))
            .nth(1)
            .expect(name)
            .split(' ')
    };
    let rgb = |name: &str| {
        let mut channels = words(name).map(|c| c.parse::<u8>().expect(name));
        let [r, g, b] = [0; 3].map(|_| channels.next().expect(name));
        Color::from_rgba8(r, g, b, 255)
    };
    let opacity = words("opacity")
        .next()
        .expect("opacity")
        .parse()
        .expect("a number");
    Row {
        placeholder: rgb("placeholder"),
        fill: rgb("fill"),
        field: rgb("input"),
        opacity,
    }
}

#[test]
fn the_saved_colours_are_the_pinned_chromiums() {
    // Given
    let pin = TOOLS
        .lines()
        .find_map(|line| line.strip_prefix("chromium.version = "))
        .expect("tools.toml pins chromium.version")
        .trim_matches('"');

    // Then: a pin moved without regenerating fails here
    let header = COMPUTED.lines().next().expect("a header");
    assert!(
        header.starts_with(&format!("# chromium.version {pin} ")),
        "computed.txt is from another Chromium than the pin {pin}: {header}; run scripts/placeholder-chromium.sh"
    );
    assert_eq!(
        COMPUTED.lines().count(),
        1 + CLASSES.len(),
        "a row per class"
    );
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
    for class in CLASSES {
        // Given: Chromium paints the fill colour, the same here
        let row = chromium(class);
        assert_eq!(row.fill, row.placeholder, "{class}: fill and colour differ");

        // When
        let painted = colours_of(&field(class), "Search");

        // Then: opaque, as Chromium's, and its colour
        assert_eq!(
            row.opacity, 1.0,
            "{class}: vapors paints no placeholder opacity"
        );
        assert_eq!(painted, [row.placeholder], "{class}");
    }
}

#[test]
fn typed_text_keeps_the_fields_own_colour_beside_a_placeholder_colour() {
    // Given: the styled field, focused
    let page = field("styled");
    let own = chromium("styled").field;
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
    let plain = chromium("plain").placeholder;
    let styled = chromium("styled").placeholder;
    assert_eq!(colours_of(&page, "Search"), [plain]);

    // When
    page.doc
        .borrow_mut()
        .set_attribute(page.plain, "class", "styled");

    // Then
    assert_eq!(colours_of(&page, "Search"), [styled]);
}
