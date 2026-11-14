//! A page is freed when it is dropped: the document, once its mounted
//! component and every handle to it are gone, is not held by anything
//! (a 10,000-row page kept 979 MB live after it was dropped, dhat,
//! docs/perf/memory-per-element-2026-11-14.md).

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use vapors_elements::{Document, Viewport};
use vapors_perf::mount_rows::{MountRows, MountRowsProps};
use vapors_signals::Signal;
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
