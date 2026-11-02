//! A native IME's composition, as the platform turns winit's events into
//! edits (`vapors-platform`'s `ime_cases.rs`, the one answer both crates
//! read): the field takes them, and one Control+Z after the commit goes
//! back to the text before the composition, as on the web.

mod support;

#[path = "../../platform/src/ime_cases.rs"]
mod ime_cases;

use support::text_page::{Page, click, control, key_with, page, selection, value};
use vapors_elements::{Document, Modifiers, TextEdit};

/// Each edit of `case` applied to the focused field.
fn composed(page: &Page, case: &[ime_cases::Edit]) {
    let units = |(a, b): (u32, u32)| (a as usize, b as usize);
    for (before, text, chosen, composing) in case {
        let edit = TextEdit {
            before: (*before).to_owned(),
            text: (*text).to_owned(),
            selection: units(*chosen),
            composing: composing.map(units),
        };
        Document::text_edited(&page.doc, &edit).unwrap_or_else(|why| panic!("{why}"));
    }
}

#[test]
fn one_undo_after_a_pinyin_commit_goes_back_to_the_text_before_it() {
    // Given: the caret after "Warm", then the pinyin composition committed
    let page = page();
    click(&page, [120.0, 12.0]);
    composed(&page, &ime_cases::PINYIN);
    assert_eq!(value(&page, page.name), "Warm啊不");

    // When
    key_with(&page, "z", control());

    // Then
    assert_eq!(value(&page, page.name), "Warm");
    assert_eq!(selection(&page, page.name), (4, 4));
}

#[test]
fn one_undo_after_a_dead_key_commit_brings_back_the_selection_it_replaced() {
    // Given: "arm" selected, then the accent composed and committed
    let page = page();
    click(&page, [120.0, 12.0]);
    let shift = Modifiers {
        shift: true,
        ..Modifiers::default()
    };
    for _ in 0..3 {
        key_with(&page, "ArrowLeft", shift);
    }
    composed(&page, &ime_cases::DEAD_KEY);
    assert_eq!(value(&page, page.name), "Wé");

    // When
    key_with(&page, "z", control());

    // Then
    assert_eq!(value(&page, page.name), "Warm");
    assert_eq!(selection(&page, page.name), (1, 4));
}
