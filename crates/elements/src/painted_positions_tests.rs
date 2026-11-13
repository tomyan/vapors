//! Every reader of a painted position puts it on the page through its
//! scroll frame: with a frame shifted, each reads its element moved by
//! the shift (IU4, `frames.rs`).

use vapors_dom::{Dom, NodeId};
use vapors_scene::{Color, Item, Rect};
use vapors_text::FontSystem;

use crate::document::Document;
use crate::kept_hits::HitsChanged;
use crate::painted::Painted;
use crate::update::Viewport;

const VIEWPORT: Viewport = Viewport {
    width: 400.0,
    height: 400.0,
    scale: 1.0,
};

/// The shift given the list's frame, device pixels at scale 1.
const SHIFT: [f32; 2] = [4.0, 20.0];

const CSS: &str = "body { margin: 0; } \
    .list { width: 300px; height: 300px; overflow: auto; } \
    .row { height: 40px; background-color: blue; } \
    .field { display: block; width: 6em; } .slider { display: block; width: 100px; }";

/// A scrolling list of a blue row, a paragraph with a span, a text
/// field and a range, painted.
struct Page {
    doc: Document,
    row: NodeId,
    span: NodeId,
    field: NodeId,
    slider: NodeId,
}

fn child(doc: &mut Document, parent: NodeId, tag: &str, class: &str) -> NodeId {
    let node = doc.create_element(tag);
    doc.set_attribute(node, "class", class);
    doc.append(parent, node);
    node
}

fn input(doc: &mut Document, parent: NodeId, kind: &str, class: &str) -> NodeId {
    let node = child(doc, parent, "input", class);
    doc.set_attribute(node, "type", kind);
    node
}

fn page() -> Page {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet("app.css", CSS).expect("parses");
    let body = doc.body();
    let list = child(&mut doc, body, "div", "list");
    let row = child(&mut doc, list, "div", "row");
    let paragraph = child(&mut doc, list, "p", "");
    let span = child(&mut doc, paragraph, "span", "");
    let word = doc.create_text("word");
    doc.append(span, word);
    let field = input(&mut doc, list, "text", "field");
    let slider = input(&mut doc, list, "range", "slider");
    doc.update(VIEWPORT).expect("updates");
    Page {
        doc,
        row,
        span,
        field,
        slider,
    }
}

/// `doc`'s paint with the list's frame shifted by `SHIFT`, its kept
/// batches to be sorted again as after a shift change.
fn shifted(doc: &mut Document) -> &Painted {
    let painted = doc.painted.as_mut().expect("painted");
    painted.frames.shift_every_frame(SHIFT);
    if let Some(kept) = painted.kept_batches.get_mut() {
        kept.unsort();
    }
    painted
}

fn moved([x, y]: [f32; 2]) -> [f32; 2] {
    [x + SHIFT[0], y + SHIFT[1]]
}

/// The blue row's quad among `items`.
fn row_quad(items: impl Iterator<Item = Item>) -> Rect {
    row_quad_in(items).expect("the row's quad")
}

/// The blue row's quad among `items`, if there.
fn row_quad_in(mut items: impl Iterator<Item = Item>) -> Option<Rect> {
    let blue = Color::from_rgba8(0, 0, 255, 255);
    items.find_map(|item| match item {
        Item::Quad(quad) if quad.background == blue => Some(quad.rect),
        _ => None,
    })
}

/// The blue row's quad among `kept`'s batches, in its frame, and the
/// frame.
fn row_batch(kept: &crate::KeptBatched) -> (Rect, crate::BatchFrame) {
    let mut batches = kept.batched.batches.iter().zip(&kept.frames);
    let found = batches.find_map(|(batch, &frame)| match &batch.run {
        vapors_scene::Run::Quads(quads) => {
            row_quad_in(quads.iter().map(|quad| Item::Quad(*quad))).map(|quad| (quad, frame))
        }
        _ => None,
    });
    found.expect("a batch with the row's quad")
}

#[test]
fn border_boxes_padding_origins_text_origins_and_tracks_move_with_the_frame() {
    // Given
    let mut page = page();
    let before = page.doc.painted().expect("painted");
    let corner = |rect: crate::layout::BoxRect| [rect.x, rect.y];
    let row = corner(before.border_box(page.row).expect("a box"));
    let span = corner(before.border_box(page.span).expect("a box"));
    let padding = before.padding_origin(page.row).expect("a box");
    let origin = before.text_origin(page.field).expect("a field");
    let track = before.range_track(page.slider).expect("a range");

    // When
    let after = shifted(&mut page.doc);

    // Then
    assert_eq!(
        corner(after.border_box(page.row).expect("a box")),
        moved(row)
    );
    assert_eq!(
        corner(after.border_box(page.span).expect("a box")),
        moved(span)
    );
    assert_eq!(after.padding_origin(page.row), Some(moved(padding)));
    assert_eq!(after.text_origin(page.field), Some(moved(origin)));
    let track_after = after.range_track(page.slider).expect("a range");
    assert_eq!(track_after.x, track.x + SHIFT[0]);
}

#[test]
fn items_batches_and_hits_move_with_the_frame() {
    // Given: the row's quad from 0 to 40, the paragraph below it
    let mut page = page();
    let view = Rect::new(0.0, 0.0, 400.0, 400.0);
    let before = page.doc.painted().expect("painted");
    let quad = row_quad(before.scene().items().map(|(item, _)| item));
    assert_ne!(before.hit([50.0, 45.0]), Some(page.row));

    // When
    let after = shifted(&mut page.doc);

    // Then
    let moved_quad = quad.offset(SHIFT);
    assert_eq!(
        row_quad(after.scene().items().map(|(item, _)| item)),
        moved_quad
    );
    let kept = after.kept_batches(view).clone();
    let (in_frame, frame) = row_batch(&kept);
    assert_eq!((in_frame, after.frame_draw(frame).shift), (quad, SHIFT));
    assert_eq!(after.hit([50.0, 45.0]), Some(page.row));
}

#[test]
fn the_hits_an_update_changed_are_put_on_the_page_through_their_frame() {
    // Given: a hit in the list's frame
    let mut page = page();
    let list_frame = page.doc.painted().expect("painted").order.thumbs[0];
    let rect = crate::layout::BoxRect {
        x: 0.0,
        y: 0.0,
        width: 300.0,
        height: 40.0,
    };

    // When
    let after = shifted(&mut page.doc);
    let changed = HitsChanged::on_page(vec![(list_frame, (page.row, rect))], &after.frames);

    // Then: (4, 20) to (304, 60), cut to the list at (0, 0) to (300, 300)
    let HitsChanged::Known(hits) = changed else {
        panic!("known hits stay known")
    };
    assert_eq!(hits[0].1.x, 4.0);
    assert_eq!(hits[0].1.y, 20.0);
    assert_eq!(hits[0].1.width, 296.0);
}
