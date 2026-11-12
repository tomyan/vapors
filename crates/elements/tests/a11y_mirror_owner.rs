//! One consumer owns the a11y takes (R6, review-v242 M1): each take
//! clears what the document collected, so a second taker would steal
//! the consumer's changes and nothing would report it. A consumer
//! attaches by name and gets the one `AxTaker`; a take by anything
//! else, or a second attach, is refused naming the owner; a detach
//! gives the takes back.

mod support;

use support::roles_page::page;
use vapors_elements::{AxError, Document};

const OWNER: &str = "ElementsApp::ax_update";

/// The roles page, updated, with `OWNER` attached.
fn owned_page() -> (Document, vapors_elements::AxTaker) {
    let mut doc = page();
    let taker = doc.ax_mirror_attach(OWNER).expect("the first attach");
    (doc, taker)
}

#[test]
fn a_take_outside_the_owner_is_refused_naming_it_and_the_fix() {
    // Given
    let (mut doc, _taker) = owned_page();

    // When
    let refused = doc.take_ax_update().expect_err("refused");

    // Then
    assert!(
        matches!(&refused, AxError::Owned { owner } if owner == OWNER),
        "{refused:?}"
    );
    let message = refused.to_string();
    assert!(
        message.contains(OWNER) && message.contains("fix:"),
        "{message}"
    );
}

#[test]
fn a_second_attach_is_refused_naming_the_owner() {
    // Given
    let (mut doc, _taker) = owned_page();

    // When
    let refused = doc
        .ax_mirror_attach("a second mirror")
        .expect_err("refused");

    // Then
    assert!(
        matches!(&refused, AxError::Owned { owner } if owner == OWNER),
        "{refused:?}"
    );
}

#[test]
fn the_owner_takes_and_nothing_is_lost_to_the_refused_take() {
    // Given: a refused take between the attach and the owner's take
    let (mut doc, taker) = owned_page();
    let _ = doc.take_ax_update();

    // When
    let update = doc
        .take_ax_update_by(&taker)
        .expect("takes")
        .expect("every");

    // Then
    assert!(update.every, "{update:?}");
    assert_eq!(
        update.nodes.len(),
        doc.ax_mirror_snapshot().expect("snapshot").len()
    );
}

#[test]
fn a_detach_gives_the_takes_back_and_the_next_take_holds_every_node() {
    // Given: the owner took once, then detached
    let (mut doc, taker) = owned_page();
    doc.take_ax_update_by(&taker).expect("takes");
    doc.ax_mirror_detach(taker).expect("the owner detaches");

    // When
    let update = doc.take_ax_update().expect("takes").expect("every");

    // Then
    assert!(update.every, "{update:?}");
}

#[test]
fn a_taker_of_another_document_is_refused() {
    // Given: two documents, each with its own taker
    let (mut doc, _own) = owned_page();
    let (_other_doc, other) = owned_page();

    // When
    let refused = doc.take_ax_update_by(&other).expect_err("refused");

    // Then
    assert!(matches!(refused, AxError::NotTaker), "{refused:?}");
    let message = refused.to_string();
    assert!(message.contains("another document"), "{message}");
}
