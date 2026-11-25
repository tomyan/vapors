//! The search input's hint (`SEARCH_PLACEHOLDER`): the input's own
//! placeholder in the spec's `MUTED`, gone while it holds text, which
//! is painted in `TEXT`; and its accessible name, the hint's text, as
//! zgui's reference app labels it.
use super::support::*;
use crate::dashboard::*;
use vapors_elements::Document;
use vapors_scene::{Color, Item};

/// The colour of each run of `text` the page paints.
fn painted(mounted: &crate::app::Mounted, text: &str) -> Vec<Color> {
    let doc = mounted.document.borrow();
    let painted = doc.painted().expect("laid out");
    let scene = painted.scene().to_scene();
    let runs = scene.items().iter().filter_map(|item| match item {
        Item::Text { text: id, .. } => Some(painted.text(*id)),
        _ => None,
    });
    runs.filter(|layout| layout.text() == text)
        .map(|layout| layout.runs()[0].color)
        .collect()
}

fn spec(color: u32) -> Color {
    let [_, r, g, b] = color.to_be_bytes();
    Color::from_rgba8(r, g, b, 255)
}

#[test]
fn the_empty_search_input_shows_its_hint_in_the_specs_muted_grey() {
    // Given, when
    let mounted = laid_out();

    // Then
    assert_eq!(painted(&mounted, SEARCH_PLACEHOLDER), [spec(MUTED)]);
}

#[test]
fn typed_search_text_is_the_specs_text_colour_and_clearing_it_brings_the_hint_back() {
    // Given: a press on the hint's text, at the input's left
    let mounted = laid_out();
    let input = by_class(&mounted.document.borrow(), "search-input")[0];
    let [x, y, _, h] = rect(&mounted.document.borrow(), input);
    click(&mounted, [x + 20.0, y + h / 2.0]);
    assert_eq!(mounted.document.borrow().active_element(), Some(input));

    // When
    let typed = |text: &str, before: &str| {
        let edit = vapors_elements::TextEdit {
            before: before.into(),
            text: text.into(),
            selection: (text.len(), text.len()),
            composing: None,
        };
        Document::text_edited(&mounted.document, &edit).expect("the field takes the edit");
        refresh(&mounted);
    };
    typed("ada", "");
    let (shown, hint) = (
        painted(&mounted, "ada"),
        painted(&mounted, SEARCH_PLACEHOLDER),
    );
    typed("", "ada");

    // Then
    assert_eq!((shown, hint), (vec![spec(TEXT)], vec![]));
    assert_eq!(painted(&mounted, SEARCH_PLACEHOLDER), [spec(MUTED)]);
}

/// The search input's ax role and name.
fn ax(mounted: &crate::app::Mounted) -> (String, String) {
    let doc = mounted.document.borrow();
    let input = by_class(&doc, "search-input")[0];
    let mut stack = doc.ax_tree().expect("laid out");
    while let Some(n) = stack.pop() {
        if n.node == input {
            return (n.role, n.name);
        }
        stack.extend(n.children);
    }
    panic!(
        "the search input is not in the ax tree (cargo test -p vapors-gui-bench-app search_hint)"
    );
}

#[test]
fn the_search_input_is_a_textbox_named_as_the_reference_names_it() {
    // Given, when
    let mounted = laid_out();

    // Then: zgui labels its search input with the hint's text
    assert_eq!(ax(&mounted), ("textbox".into(), SEARCH_PLACEHOLDER.into()));
}
