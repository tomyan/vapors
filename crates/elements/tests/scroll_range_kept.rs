//! A scroll container's range is kept when it only scrolled (4g,
//! `docs/plans/splice-layout.md` section 12): the settle clamps to the
//! range from its last settle, measured again only when the update marks
//! the container laid out or built again. Every way the range can change
//! marks it so; the placement check, which settles every container
//! afresh on each debug update, guards the kept range.

use crate::support;

use support::{document, element};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Viewport};

const VIEW: Viewport = Viewport {
    width: 400.0,
    height: 600.0,
    scale: 1.0,
};

/// Ten 50 px items and a one-line note (20 px) in a scroller as tall as
/// its parent, which fills the 600 px body under a 300 px header: 300 px
/// at [`VIEW`], a range of 220 px. The first item holds a 50 px child
/// that can overflow it; the scroller holds a 10 px absolutely
/// positioned child.
const CSS: &str = "html, body { margin: 0; height: 100%; } \
    body { display: flex; flex-direction: column; } .header { height: 300px; flex: none; } \
    .header.short { height: 200px; } .outer { flex: 1; min-height: 0; } \
    .outer.tall { flex: none; height: 400px; } \
    .scroller { height: 100%; overflow: auto; box-sizing: border-box; position: relative; } \
    .scroller.padded { padding-bottom: 40px; } .scroller.bordered { border-bottom: 10px solid #000; } \
    .item { height: 50px; } .item.big { height: 90px; } .note { width: 100px; line-height: 20px; } \
    img { display: block; width: 10px; } .over { height: 50px; } .over.grown { height: 900px; } \
    .abs { position: absolute; top: 0; left: 0; width: 10px; height: 10px; } \
    .abs.tall { height: 900px; }";

/// The page: the document, the viewport it is updated at, the header,
/// the scroller's parent, the scroller, its items, the note's text, the
/// first item's child and the absolutely positioned child.
struct Page {
    doc: Document,
    view: Viewport,
    header: NodeId,
    outer: NodeId,
    scroller: NodeId,
    items: Vec<NodeId>,
    note: NodeId,
    over: NodeId,
    abs: NodeId,
}

fn page() -> Page {
    let mut doc = document();
    doc.add_stylesheet("page.css", CSS).expect("parses");
    let body = doc.body();
    let header = element(&mut doc, body, "div", "header");
    let outer = element(&mut doc, body, "div", "outer");
    let scroller = element(&mut doc, outer, "div", "scroller");
    let items: Vec<NodeId> = (0..10)
        .map(|_| element(&mut doc, scroller, "div", "item"))
        .collect();
    let over = element(&mut doc, items[0], "div", "over");
    let holder = element(&mut doc, scroller, "div", "note");
    let note = doc.create_text("a");
    doc.append(holder, note);
    let abs = element(&mut doc, scroller, "div", "abs");
    doc.update(VIEW).expect("updates");
    Page {
        doc,
        view: VIEW,
        header,
        outer,
        scroller,
        items,
        note,
        over,
        abs,
    }
}

impl Page {
    /// Updates at the page's viewport.
    fn update(&mut self) {
        self.doc.update(self.view).expect("updates");
    }

    /// Scrolls the scroller as far down as it goes, and updates.
    fn scroll_to_end(&mut self) {
        self.doc.scroll_to(self.scroller, [0.0, 1.0e6]);
        self.update();
    }

    /// The scroller's offset down.
    fn offset(&self) -> f32 {
        self.doc.scroll_offset(self.scroller).expect("scrolls")[1]
    }

    /// The ranges the last update measured.
    fn measured(&self) -> usize {
        self.doc.update_counts().scroll_ranges_measured
    }
}

#[test]
fn a_scroll_alone_measures_no_range() {
    // Given
    let mut page = page();

    // When
    page.doc.scroll_node_by(page.scroller, [0.0, 120.0]);
    page.update();

    // Then: the offset is clamped to the kept range, none measured
    assert_eq!(page.measured(), 0);
    assert!((page.offset() - 120.0).abs() < 1e-3, "{}", page.offset());
}

#[test]
fn a_scroll_past_the_end_is_clamped_to_the_kept_range() {
    // Given
    let mut page = page();

    // When
    page.scroll_to_end();

    // Then
    assert_eq!(page.measured(), 0);
    assert!((page.offset() - 220.0).abs() < 1e-3, "{}", page.offset());
}

/// The header 100 px shorter: the scroller's parent, which fills the
/// body under it, and so the scroller, 100 px taller, by layout alone
/// (the header is restyled; the parent and the scroller are laid out
/// again, not built again).
fn header_shorter(page: &mut Page) {
    page.doc.set_attribute(page.header, "class", "header short");
}

/// Each change that moves the scroller's range: its own size (an
/// ancestor restyled, or resized by layout alone; the viewport resized),
/// its content's size (an item restyled, inserted or removed; a
/// text in it wrapping to more lines), its padding and its border. An
/// image loading is `image_loaded`, below.
/// A change to the page, named.
type Change = (&'static str, fn(&mut Page));

fn range_changes() -> Vec<Change> {
    vec![
        ("an ancestor restyled taller", |page| {
            page.doc.set_attribute(page.outer, "class", "outer tall");
        }),
        ("an ancestor resized by layout", header_shorter),
        ("the viewport taller", |page| {
            page.view = Viewport {
                height: 700.0,
                ..VIEW
            };
        }),
        ("an item restyled taller", |page| {
            page.doc.set_attribute(page.items[4], "class", "item big");
        }),
        (
            "a descendant restyled to overflow a fixed-height item",
            |page| {
                page.doc.set_attribute(page.over, "class", "over grown");
            },
        ),
        ("an absolutely positioned child taller", |page| {
            page.doc.set_attribute(page.abs, "class", "abs tall");
        }),
        ("a text wrapping to more lines", |page| {
            page.doc
                .set_text(page.note, "a b c d e f g h i j k l m n o p");
        }),
        ("an item inserted", |page| {
            let item = page.doc.create_element("div");
            page.doc.set_attribute(item, "class", "item");
            page.doc
                .insert_before(page.scroller, item, Some(page.items[2]));
        }),
        ("an item removed", |page| page.doc.remove(page.items[7])),
        ("padding added", |page| {
            page.doc
                .set_attribute(page.scroller, "class", "scroller padded");
        }),
        ("a border added", |page| {
            page.doc
                .set_attribute(page.scroller, "class", "scroller bordered");
        }),
    ]
}

#[test]
fn every_change_to_the_range_measures_it_again() {
    for (case, change) in range_changes() {
        // Given: scrolled to the end, the range kept
        let mut page = page();
        page.scroll_to_end();
        let before = page.offset();

        // When
        change(&mut page);
        page.update();

        // Then: the range was measured, and the offset moved with it (the
        // placement check also compares it with a fresh settle)
        assert!(page.measured() >= 1, "{case}: {}", page.measured());
        page.scroll_to_end();
        assert!((page.offset() - before).abs() > 1.0, "{case}: {before}");
    }
}

#[test]
fn a_scroll_and_a_range_change_in_one_update_measure_the_range() {
    // Given: scrolled to the end, the range kept
    let mut page = page();
    page.scroll_to_end();
    let before = page.offset();

    // When: scrolled up and an item made taller, then one update
    page.doc.scroll_node_by(page.scroller, [0.0, -50.0]);
    page.doc.set_attribute(page.items[4], "class", "item big");
    page.update();

    // Then: the change's mark wins over the scroll's, and the end moved
    assert_eq!(page.measured(), 1);
    page.scroll_to_end();
    assert!(
        (page.offset() - before - 40.0).abs() < 1e-3,
        "{before} then {}",
        page.offset()
    );
}

#[test]
fn an_ancestor_resize_that_changes_only_the_scrollers_height_clamps_the_offset() {
    // Given: scrolled to the end, 220 px
    let mut page = page();
    page.scroll_to_end();

    // When: the scroller 100 px taller by layout alone
    header_shorter(&mut page);
    page.update();

    // Then: the range is 120 px, and the offset is clamped to it
    assert!((page.offset() - 120.0).abs() < 1e-3, "{}", page.offset());
}

/// An image `height` pixels tall and one wide, as `register_image` takes it.
fn image(height: u32) -> vapors_scene::RgbaImage {
    let size = vapors_scene::Size { width: 1, height };
    let rgba = vec![255; 4 * height as usize];
    vapors_scene::RgbaImage { size, rgba }
}

/// The page with an image at the end of the items showing `late.png`,
/// registered at one pixel square (drawn 10 px wide: 10 px tall), as an
/// app registers a placeholder until the image has loaded.
fn page_with_image() -> Page {
    let mut page = page();
    page.doc
        .register_image("late.png", image(1))
        .expect("registers");
    let img = element(&mut page.doc, page.scroller, "img", "");
    page.doc.set_attribute(img, "src", "late.png");
    page.update();
    page
}

/// The image loaded: `late.png` registered again, 1 by 10 pixels, drawn
/// 10 px wide: 100 px tall.
fn image_loaded(page: &mut Page) {
    page.doc
        .register_image("late.png", image(10))
        .expect("registers");
}

#[test]
fn an_image_loading_inside_the_scroller_grows_its_range() {
    // Given: scrolled to the end, the placeholder 10 px tall
    let mut page = page_with_image();
    page.scroll_to_end();
    let before = page.offset();

    // When
    image_loaded(&mut page);
    page.update();
    page.scroll_to_end();

    // Then: the range grew by 90 px
    assert!(
        (page.offset() - before - 90.0).abs() < 1e-3,
        "{before} then {}",
        page.offset()
    );
}

#[test]
fn an_image_loading_measures_the_range_again() {
    // Given
    let mut page = page_with_image();
    page.scroll_to_end();

    // When
    image_loaded(&mut page);
    page.update();

    // Then
    assert!(page.measured() >= 1, "{}", page.measured());
}
