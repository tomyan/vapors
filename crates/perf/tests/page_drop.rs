//! A page is freed when it is dropped: the document, once its mounted
//! component and every handle to it are gone, is not held by anything,
//! whatever owner was current when it was mounted (before #353 a
//! dropped 10,000-row page left 443.7 MB live, measured by
//! vapors-perf-memory, docs/perf/memory-per-element-2026-11-14.md).

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use vapors_elements::{Document, Viewport};
use vapors_perf::mount_rows::{MountRows, MountRowsProps};
use vapors_signals::{Owner, Signal};
use vapors_text::FontSystem;

const VIEWPORT: Viewport = Viewport {
    width: 640.0,
    height: 400.0,
    scale: 1.0,
};

/// A mounted page of `rows` rows, updated once; a weak handle to its
/// document, which is all the test keeps once the page is dropped.
fn page(rows: u32) -> (Rc<RefCell<Document>>, MountRows, Weak<RefCell<Document>>) {
    let doc = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
    let body = doc.borrow().body();
    let data = (0..rows)
        .map(|i| (i, format!("Row {i}"), format!("{i}")))
        .collect();
    let mounted = MountRows::mount(
        &doc,
        body,
        MountRowsProps {
            rows: Signal::new(data),
        },
    );
    doc.borrow_mut().update(VIEWPORT).expect("updates");
    let weak = Rc::downgrade(&doc);
    (doc, mounted, weak)
}

#[test]
fn a_dropped_page_s_document_is_freed() {
    // Given
    let (doc, mounted, weak) = page(5);

    // When
    drop(mounted);
    drop(doc);

    // Then
    assert!(
        weak.upgrade().is_none(),
        "the document is still held by {} handles after its page was dropped",
        weak.strong_count()
    );
}

/// The page's document is freed once the page and the doc are dropped.
fn assert_freed(doc: Rc<RefCell<Document>>, mounted: MountRows, weak: &Weak<RefCell<Document>>) {
    drop(mounted);
    drop(doc);
    assert!(
        weak.upgrade().is_none(),
        "the document is still held by {} handles after its page was dropped",
        weak.strong_count()
    );
}

#[test]
fn a_page_mounted_under_a_long_lived_owner_is_freed_when_dropped() {
    // Given: an owner made current for the thread, as an app's start-up may
    let ambient = Owner::new();
    ambient.set();

    // When
    let (doc, mounted, weak) = page(5);

    // Then
    assert_freed(doc, mounted, &weak);
}

#[test]
fn a_page_mounted_while_the_current_owner_is_disposed_is_freed_when_dropped() {
    // Given: inside `outer`, its child `inner` restores `outer` as current
    // after `outer` was disposed, so the current owner is a dead one
    let outer = Owner::new();
    let inner = outer.with(Owner::new);

    // When
    let (doc, mounted, weak) = outer.with(|| {
        inner.with(|| outer.dispose());
        page(5)
    });

    // Then
    assert_freed(doc, mounted, &weak);
}
