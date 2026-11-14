//! The boxes a full build keeps hold no spare slots (property 8:
//! grown by doubling, 40,002 boxes of about 2.8 KB each held 65,536).

use vapors_dom::Dom;
use vapors_text::FontSystem;

use crate::document::Document;
use crate::update::Viewport;

#[test]
fn a_full_build_keeps_its_boxes_in_as_many_slots_as_boxes() {
    // Given: html, body and five divs, seven boxes, grown to eight slots
    let mut doc = Document::new(FontSystem::bundle_only());
    let body = doc.body();
    for _ in 0..5 {
        let div = doc.create_element("div");
        doc.append(body, div);
    }

    // When
    let viewport = Viewport {
        width: 200.0,
        height: 100.0,
        scale: 1.0,
    };
    doc.update(viewport).expect("updates");

    // Then
    let boxes = &doc.boxes.placed.boxes;
    assert_eq!((boxes.len(), boxes.capacity()), (7, 7));
}
