//! `bind:this` in a `{for}` row (`src/refs.wisp`): each row's node in a
//! `BTreeMap` under its key while the row is mounted; a removed row's
//! key goes, and a reused row keeps its entry.

#[path = "support/refs.rs"]
mod refs;

use std::collections::BTreeMap;

use refs::{mounted, row};

#[test]
fn each_row_is_under_its_key() {
    // When
    let (dom, refs, _) = mounted(&["a", "b"]);

    // Then
    let expected = BTreeMap::from([
        ("a".to_owned(), row(&dom, "a")),
        ("b".to_owned(), row(&dom, "b")),
    ]);
    assert_eq!(refs.rows.get(), expected);
}

#[test]
fn a_removed_row_takes_only_its_own_key() {
    // Given
    let (dom, refs, names) = mounted(&["a", "b"]);
    let b = row(&dom, "b");

    // When
    names.set(vec!["b".into()]);

    // Then
    assert_eq!(refs.rows.get(), BTreeMap::from([("b".to_owned(), b)]));
}

#[test]
fn reordered_rows_keep_their_nodes() {
    // Given
    let (dom, refs, names) = mounted(&["a", "b"]);
    let before = refs.rows.get();

    // When
    names.set(vec!["b".into(), "a".into()]);

    // Then: the rows were reused, with their entries
    assert_eq!(refs.rows.get(), before);
    assert_eq!(before["a"], row(&dom, "a"));
}

#[test]
fn every_row_gone_leaves_an_empty_map() {
    // Given
    let (_dom, refs, names) = mounted(&["a", "b"]);

    // When
    names.set(Vec::new());

    // Then
    assert!(refs.rows.get().is_empty());
}
