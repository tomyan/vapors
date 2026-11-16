//! GPU tier: a framed lane's rows drawn by batches that span them
//! (`docs/plans/splice-layout.md` section 11.2, 4f): the rows' backgrounds
//! are one draw, each row moved by its offset in the batch's table, so
//! after a row is put in at the top every row's pixels are where the
//! insert puts it in device pixels (scale 2).

#[path = "support/surface.rs"]
mod support;

use support::{BLUE, RED, Rig, WHITE, document, pixel, update};
use vapors_dom::Dom;
use vapors_elements::Document;
use vapors_scene::{RgbaImage, Size};

/// Rows 2 CSS px tall, red and blue in turn: 4 device px each.
const CSS: &str = ".row { height: 2px; background-color: #f00; } .blue { background-color: #00f; }";

/// Device pixels per CSS pixel.
const SCALE: f32 = 2.0;

#[test]
fn gpu_rows_put_after_an_insert_draw_in_one_batch_each_at_its_offset() {
    // Given: 40 rows, red and blue in turn, drawn once
    let rows: Vec<_> = (0..40)
        .map(|at| (None, "div", if at % 2 == 0 { "row" } else { "row blue" }))
        .collect();
    let mut doc = document(CSS, &rows);
    update(&mut doc, SCALE);
    let mut rig = Rig::new();
    rig.draw(&doc).expect("paints");

    // When: a blue row put in at the top
    let row = doc.create_element("div");
    doc.set_attribute(row, "class", "row blue");
    let first = doc.children(doc.body())[0];
    doc.insert_before(doc.body(), row, Some(first));
    update(&mut doc, SCALE);
    let pixels = rig.draw(&doc).expect("paints");

    // Then: the lane is framed, and the page's backgrounds (html and
    // body, white) are one draw and the rows' another
    assert!(
        doc.update_counts().lanes_framed > 0,
        "{:?}",
        doc.update_counts()
    );
    assert_eq!(rig.painter.frame_counts().quads.draw_calls, 2);
    // and each of the eight rows in the 32 px target is where the insert
    // puts it: the new blue row, then red, blue, ...
    let colours: Vec<_> = (0..8).map(|row| pixel(&pixels, 2, row * 4 + 2)).collect();
    let expected: Vec<_> = (0..8)
        .map(|row| if row % 2 == 0 { BLUE } else { RED })
        .collect();
    assert_eq!(colours, expected);
}

/// Rows of images 2 CSS px tall: 4 device px each.
const IMAGE_CSS: &str = "img { display: block; width: 8px; height: 2px; }";

/// `doc` with a one-pixel image of `rgba` registered as `name`.
fn register(doc: &mut Document, name: &str, rgba: [u8; 4]) {
    let size = Size {
        width: 1,
        height: 1,
    };
    let image = RgbaImage {
        size,
        rgba: rgba.to_vec(),
    };
    doc.register_image(name, image).expect("registers");
}

/// An image row showing `name`, put in at the top of the body.
fn image_row_at_top(doc: &mut Document, name: &str) {
    let row = doc.create_element("img");
    doc.set_attribute(row, "src", name);
    let first = doc.children(doc.body())[0];
    doc.insert_before(doc.body(), row, Some(first));
}

#[test]
fn gpu_image_rows_put_after_an_insert_draw_each_at_its_rows_offset() {
    // Given: 40 image rows, red and blue in turn, drawn once
    let rows: Vec<_> = (0..40).map(|_| (None, "img", "")).collect();
    let mut doc = document(IMAGE_CSS, &rows);
    register(&mut doc, "red.png", RED);
    register(&mut doc, "blue.png", BLUE);
    for (at, row) in doc.children(doc.body()).into_iter().enumerate() {
        let name = if at % 2 == 0 { "red.png" } else { "blue.png" };
        doc.set_attribute(row, "src", name);
    }
    update(&mut doc, SCALE);
    let mut rig = Rig::new();
    rig.draw(&doc).expect("paints");

    // When: a blue image row put in at the top
    image_row_at_top(&mut doc, "blue.png");
    update(&mut doc, SCALE);
    let pixels = rig.draw(&doc).expect("paints");

    // Then: the lane is framed, and each row is where the insert puts it
    assert!(
        doc.update_counts().lanes_framed > 0,
        "{:?}",
        doc.update_counts()
    );
    let colours: Vec<_> = (0..8).map(|row| pixel(&pixels, 2, row * 4 + 2)).collect();
    let expected: Vec<_> = (0..8)
        .map(|row| if row % 2 == 0 { BLUE } else { RED })
        .collect();
    assert_eq!(colours, expected);
}

/// Rows 4 CSS px wide whose shadows, red and blue in turn, fall 8 px to
/// their right: device x 16 to 24 shows only shadows.
const SHADOW_CSS: &str = ".row { width: 4px; height: 2px; box-shadow: 8px 0 0 0 #f00; } \
    .blue { box-shadow: 8px 0 0 0 #00f; }";

#[test]
fn gpu_shadow_rows_put_after_an_insert_draw_each_at_its_rows_offset() {
    // Given: 40 rows, red and blue in turn, drawn once
    let rows: Vec<_> = (0..40)
        .map(|at| (None, "div", if at % 2 == 0 { "row" } else { "row blue" }))
        .collect();
    let mut doc = document(SHADOW_CSS, &rows);
    update(&mut doc, SCALE);
    let mut rig = Rig::new();
    rig.draw(&doc).expect("paints");

    // When: a blue row put in at the top
    let row = doc.create_element("div");
    doc.set_attribute(row, "class", "row blue");
    let first = doc.children(doc.body())[0];
    doc.insert_before(doc.body(), row, Some(first));
    update(&mut doc, SCALE);
    let pixels = rig.draw(&doc).expect("paints");

    // Then: each row's shadow is where the insert puts the row
    let colours: Vec<_> = (0..8).map(|row| pixel(&pixels, 18, row * 4 + 2)).collect();
    let expected: Vec<_> = (0..8)
        .map(|row| if row % 2 == 0 { BLUE } else { RED })
        .collect();
    assert_eq!(colours, expected);
}

/// Rows 2 CSS px tall, each a box clipping its overflow over a child
/// twice its size, red and blue in turn: only the clip shows each row.
const CLIPPED_CSS: &str = ".row { height: 2px; } \
    .clip { width: 4px; height: 2px; overflow: hidden; } \
    .fill { width: 8px; height: 4px; background-color: #f00; } \
    .blue .fill { background-color: #00f; }";

#[test]
fn gpu_rows_with_a_clipped_box_each_show_it_where_the_insert_puts_the_row() {
    // Given: 40 rows, each a clip over a fill, red and blue in turn
    let mut elements = Vec::new();
    for at in 0..40 {
        let row = elements.len();
        elements.push((None, "div", if at % 2 == 0 { "row" } else { "row blue" }));
        elements.push((Some(row), "div", "clip"));
        elements.push((Some(row + 1), "div", "fill"));
    }
    let mut doc = document(CLIPPED_CSS, &elements);
    update(&mut doc, SCALE);
    let mut rig = Rig::new();
    rig.draw(&doc).expect("paints");

    // When: a blue row put in at the top
    let row = doc.create_element("div");
    doc.set_attribute(row, "class", "row blue");
    let first = doc.children(doc.body())[0];
    doc.insert_before(doc.body(), row, Some(first));
    update(&mut doc, SCALE);
    let pixels = rig.draw(&doc).expect("paints");

    // Then: the new row is empty; each row after it shows its fill inside
    // its clip, and nothing past the clip's right edge
    assert!(
        doc.update_counts().lanes_framed > 0,
        "{:?}",
        doc.update_counts()
    );
    let colours: Vec<_> = (1..8).map(|row| pixel(&pixels, 2, row * 4 + 2)).collect();
    let expected: Vec<_> = (1..8)
        .map(|row| if row % 2 == 0 { BLUE } else { RED })
        .collect();
    assert_eq!(colours, expected);
    assert_eq!(pixel(&pixels, 12, 6), WHITE);
}

/// Red rows 2 CSS px tall, a blue one among them; a spaced row has 2 px
/// above it, or below it.
const SPACED_CSS: &str = ".row { height: 2px; background-color: #f00; } \
    .blue { background-color: #00f; } .above { margin-top: 2px; } \
    .below { margin-bottom: 2px; }";

#[test]
fn gpu_a_row_kept_in_view_moves_when_its_margin_moves_from_above_it_to_below() {
    // Given: 40 rows, the second blue with a margin above it, framed (a
    // lane is framed from its first written update: a row put in at the
    // end, out of view) and drawn
    let rows: Vec<_> = (0..40)
        .map(|at| (None, "div", if at == 1 { "row blue above" } else { "row" }))
        .collect();
    let mut doc = document(SPACED_CSS, &rows);
    update(&mut doc, SCALE);
    let last = doc.create_element("div");
    doc.set_attribute(last, "class", "row");
    doc.append(doc.body(), last);
    update(&mut doc, SCALE);
    assert!(
        doc.update_counts().lanes_framed > 0,
        "{:?}",
        doc.update_counts()
    );
    let mut rig = Rig::new();
    rig.draw(&doc).expect("paints");

    // When: its margin moves below it; every row's paint and the rows in
    // view are the same, so the batch keeps its key and only the blue
    // row's offset changes
    let second = doc.children(doc.body())[1];
    doc.set_attribute(second, "class", "row blue below");
    update(&mut doc, SCALE);
    let pixels = rig.draw(&doc).expect("paints");

    // Then: the blue row is 4 device px higher, the gap below it
    let column: Vec<_> = [2, 6, 10, 14].map(|y| pixel(&pixels, 2, y)).to_vec();
    assert_eq!(column, [RED, BLUE, WHITE, RED]);
}
