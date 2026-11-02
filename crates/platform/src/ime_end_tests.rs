//! When a composition ends other than by its commit: a cancelled
//! preedit, a selection moved out of it, another field.

use super::*;

/// "Warm" (field `id`) with `selection`.
fn field(id: u64, text: &str, selection: (u32, u32)) -> TextField {
    TextField {
        id,
        text: text.into(),
        selection,
        area: [0.0; 4],
    }
}

/// Composing "a" after "Warm" in field 1.
fn composing_a() -> Composition {
    let mut composition = Composition::default();
    composition.set_field(Some(field(1, "Warm", (4, 4))));
    drop(composition.edit(&Ime::Preedit("a".into(), Some((1, 1)))));
    composition.set_field(Some(field(1, "Warma", (5, 5))));
    composition
}

fn text_of(event: Option<Event>) -> String {
    match event {
        Some(Event::TextEdited { text, .. }) => text,
        other => panic!("not an edit: {other:?}"),
    }
}

#[test]
fn a_cleared_preedit_no_commit_follows_is_applied_before_the_next_event() {
    // Given: the IME cleared its preedit (cancelled)
    let mut composition = composing_a();
    assert!(
        composition
            .edit(&Ime::Preedit(String::new(), None))
            .is_none()
    );

    // When: the next event that is not the IME's
    let flushed = composition.flush();

    // Then: the preedit goes, once
    assert_eq!(text_of(flushed), "Warm");
    assert!(composition.flush().is_none());
}

#[test]
fn a_selection_moved_out_of_the_composition_ends_it_keeping_its_text() {
    // Given
    let mut composition = composing_a();

    // When: a click at 2, inside the field, outside the composing range
    let ended = composition.set_field(Some(field(1, "Warma", (2, 2))));
    let typed = text_of(composition.edit(&Ime::Commit("b".into())));

    // Then: the composition ended, and a commit goes at the caret
    assert!(ended);
    assert_eq!(typed, "Wabrma");
}

#[test]
fn a_selection_inside_the_composition_keeps_it() {
    // Given
    let mut composition = composing_a();

    // When: the app's echo of the preedit's own caret
    let ended = composition.set_field(Some(field(1, "Warma", (4, 5))));

    // Then
    assert!(!ended);
}

#[test]
fn another_field_with_the_same_text_ends_the_composition() {
    // Given
    let mut composition = composing_a();

    // When: focus moves to field 2, which also holds "Warma"
    let ended = composition.set_field(Some(field(2, "Warma", (5, 5))));
    let typed = text_of(composition.edit(&Ime::Commit("b".into())));

    // Then: the commit goes at field 2's caret, not over field 1's range
    assert!(ended);
    assert_eq!(typed, "Warmab");
}
