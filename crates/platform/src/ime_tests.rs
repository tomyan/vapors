//! IME events over a field as the edits the app takes: the commit one
//! edit with the preedit it replaces (`ime_cases.rs`, which the
//! elements' undo test reads too).

use super::*;

#[path = "ime_cases.rs"]
mod cases;

/// An edit as `ime_cases` writes it.
fn case(edit: &cases::Edit) -> Edit {
    (edit.0.into(), edit.1.into(), edit.2, edit.3)
}

/// "Warm" with the caret at its end.
fn warm() -> Composition {
    let mut composition = Composition::default();
    composition.set_field(Some(TextField {
        id: 1,
        text: "Warm".into(),
        selection: (4, 4),
        area: [0.0; 4],
    }));
    composition
}

/// An edit's before, text, selection and composing range.
type Edit = (String, String, (u32, u32), Option<(u32, u32)>);

fn edited(event: Option<Event>) -> Edit {
    match event {
        Some(Event::TextEdited {
            before,
            text,
            selection,
            composing,
        }) => (before, text, selection, composing),
        other => panic!("not an edit: {other:?}"),
    }
}

#[test]
fn a_pinyin_composition_is_in_the_value_then_committed_in_one_edit() {
    // Given
    let mut composition = warm();

    // When: winit's own pinyin example after "Warm"
    let a = edited(composition.edit(&Ime::Preedit("a".into(), Some((1, 1)))));
    let ab = edited(composition.edit(&Ime::Preedit("a b".into(), Some((3, 3)))));
    let cleared = composition.edit(&Ime::Preedit(String::new(), None));
    let committed = edited(composition.edit(&Ime::Commit("啊不".into())));

    // Then: the clear before the commit is held, and the commit replaces
    // the preedit in one edit
    assert!(cleared.is_none(), "{cleared:?}");
    assert_eq!(
        vec![a, ab, committed],
        cases::PINYIN.iter().map(case).collect::<Vec<_>>()
    );
}

#[test]
fn a_dead_key_commit_replaces_the_selection_in_one_edit() {
    // Given: "arm" selected
    let mut composition = Composition::default();
    composition.set_field(Some(TextField {
        id: 1,
        text: "Warm".into(),
        selection: (4, 1),
        area: [0.0; 4],
    }));

    // When
    let accent = edited(composition.edit(&Ime::Preedit("`".into(), Some((0, 0)))));
    let cleared = composition.edit(&Ime::Preedit(String::new(), None));
    let committed = edited(composition.edit(&Ime::Commit("é".into())));

    // Then: the accent composes over the selection, the caret before it
    assert!(cleared.is_none(), "{cleared:?}");
    assert_eq!(
        vec![accent, committed],
        cases::DEAD_KEY.iter().map(case).collect::<Vec<_>>()
    );
}

#[test]
fn nothing_changes_with_no_field_or_nothing_composing() {
    // Given
    let mut none = Composition::default();
    let mut idle = warm();

    // Then
    assert!(none.edit(&Ime::Commit("a".into())).is_none());
    assert!(idle.edit(&Ime::Preedit(String::new(), None)).is_none());
    assert!(idle.edit(&Ime::Disabled).is_none());
    assert!(idle.edit(&Ime::Enabled).is_none());
}

#[test]
fn losing_the_field_ends_the_composition() {
    // Given: composing "a"
    let mut composition = warm();
    drop(composition.edit(&Ime::Preedit("a".into(), Some((1, 1)))));

    // When: focus leaves, then a field is set again
    composition.set_field(None);
    composition.set_field(Some(TextField {
        id: 1,
        text: "Warma".into(),
        selection: (5, 5),
        area: [0.0; 4],
    }));
    let typed = edited(composition.edit(&Ime::Commit("b".into())));

    // Then: the commit goes at the caret, not over the old composing range
    assert_eq!(typed.1, "Warmab");
}

#[test]
fn a_field_set_with_another_text_ends_the_composition() {
    // Given: composing "a" after "Warm"
    let mut composition = warm();
    drop(composition.edit(&Ime::Preedit("a".into(), Some((1, 1)))));

    // When: the app sets "Cool" (it refused the edit, or a signal set it)
    composition.set_field(Some(TextField {
        id: 1,
        text: "Cool".into(),
        selection: (4, 4),
        area: [0.0; 4],
    }));
    let typed = edited(composition.edit(&Ime::Commit("b".into())));

    // Then: the commit goes at the caret
    assert_eq!(typed.1, "Coolb");
}
