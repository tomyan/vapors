//! How a field's edits group into undo steps, as Chromium 155 groups
//! them (review-v92's probes; pinned against Chromium by the corpus's
//! `undo.events`): each case is a run of edits, and how many undo steps
//! they make.

use super::{EditKind, History};
use EditKind::{
    DeleteBackward, DeleteForward, DeleteWordBackward, DeleteWordForward, Insert, Paste,
};

/// The undo steps `kinds`, applied one after another, make.
fn steps(kinds: &[EditKind]) -> usize {
    let mut history = History::default();
    for (at, kind) in kinds.iter().enumerate() {
        history.edited(*kind, (at.to_string(), 0..0), ((at + 1).to_string(), 0..0));
    }
    std::iter::from_fn(|| history.undo()).count()
}

#[test]
#[allow(clippy::too_many_lines)] // one table of edit sequences, a case per grouping rule
fn edits_group_into_steps_as_chromium_groups_them() {
    let cases: &[(&str, &[EditKind], usize)] = &[
        ("typing is one step", &[Insert, Insert, Insert], 1),
        (
            "Backspaces are one step",
            &[DeleteBackward, DeleteBackward],
            1,
        ),
        (
            "a deletion after typing is a new step",
            &[Insert, DeleteBackward],
            2,
        ),
        (
            "typing after a deletion joins it",
            &[DeleteForward, Insert],
            1,
        ),
        (
            "Delete then Backspace are two steps",
            &[DeleteForward, DeleteBackward],
            2,
        ),
        (
            "a Delete joins an open Backspace run",
            &[DeleteBackward, DeleteForward],
            1,
        ),
        (
            "a word deletion starts its own step",
            &[DeleteWordBackward, DeleteWordBackward],
            2,
        ),
        (
            "a word deletion after Backspace is new",
            &[DeleteBackward, DeleteWordBackward],
            2,
        ),
        (
            "a word deletion after Delete is new",
            &[DeleteForward, DeleteWordForward],
            2,
        ),
        (
            "Backspace joins a word deletion back",
            &[DeleteWordBackward, DeleteBackward],
            1,
        ),
        (
            "Delete joins a word deletion forward",
            &[DeleteWordForward, DeleteForward],
            1,
        ),
        (
            "typing joins a word deletion",
            &[DeleteWordBackward, Insert],
            1,
        ),
        ("a paste is a step alone", &[Insert, Paste, Insert], 3),
    ];
    for (case, kinds, expected) in cases {
        assert_eq!(steps(kinds), *expected, "{case}");
    }
}

#[test]
fn the_history_keeps_at_most_its_limit_of_steps() {
    // Given: more steps than the limit, each its own (a paste)
    let mut history = History::default();
    for at in 0..=super::LIMIT {
        history.edited(Paste, (at.to_string(), 0..0), ((at + 1).to_string(), 0..0));
    }

    // When
    let undone = std::iter::from_fn(|| history.undo()).count();

    // Then: the oldest went; the last undo reaches the second text
    assert_eq!(undone, super::LIMIT);
}
