//! A document's shapes counted per update: an acquire made between two
//! updates is not the second update's tessellation.

use vapors_shapes::{FillRule, IconPath, Paint};

use crate::{Document, Viewport};

#[test]
fn an_acquire_between_updates_is_not_the_next_updates_tessellation() {
    // Given: a document updated once
    let mut doc = Document::new(vapors_text::FontSystem::bundle_only());
    let viewport = Viewport {
        width: 100.0,
        height: 100.0,
        scale: 1.0,
    };
    doc.update(viewport).expect("updates");

    // When: a path tessellated outside an update, then an update
    let path = IconPath::parse("M0 0L4 0L0 4z").expect("parses");
    let fill = Paint::Fill(FillRule::NonZero);
    doc.shapes
        .borrow_mut()
        .cache
        .acquire("test", &path, fill, 1.0)
        .expect("acquired");
    doc.update(viewport).expect("updates");

    // Then
    assert_eq!(doc.update_counts().shapes_tessellated, 0);
}
