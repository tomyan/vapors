//! GPU tier: clips inside the rows of a framed lane
//! (`docs/plans/splice-layout.md` section 11.2, 4f): each clip is the
//! scissor of its own row after an insert moves the rows, for a box, an
//! image and a clip nested in another (#407's review, H1). Pixels are
//! checked against the colours each row must show: a painter that kept
//! nothing batches the same way, so it would not tell.

use crate::support_surface as support;

use support::{BLUE, RED, Rig, WHITE, document, pixel, update};
use vapors_dom::Dom;
use vapors_elements::Document;
use vapors_scene::{RgbaImage, Size};

/// Device pixels per CSS pixel.
const SCALE: f32 = 2.0;

/// 40 rows each holding a chain of `children` (`(tag, class)`, each in
/// the one before), red and blue in turn by the row's class, drawn once; then a row of class
/// `row blue` and nothing in it put in at the top, and drawn again: the
/// pixels.
fn drawn_after_an_insert(
    css: &str,
    children: &[(&str, &str)],
    prepare: impl Fn(&mut Document),
) -> Vec<u8> {
    let mut elements = Vec::new();
    for at in 0..40 {
        let row = elements.len();
        elements.push((None, "div", if at % 2 == 0 { "row" } else { "row blue" }));
        for (depth, (tag, class)) in children.iter().enumerate() {
            elements.push((Some(row + depth), *tag, *class));
        }
    }
    let mut doc = document(css, &elements);
    prepare(&mut doc);
    update(&mut doc, SCALE);
    let mut rig = Rig::new();
    rig.draw(&doc).expect("paints");
    let row = doc.create_element("div");
    doc.set_attribute(row, "class", "row blue");
    let first = doc.children(doc.body())[0];
    doc.insert_before(doc.body(), row, Some(first));
    update(&mut doc, SCALE);
    assert!(
        doc.update_counts().lanes_framed > 0,
        "{:?}",
        doc.update_counts()
    );
    rig.draw(&doc).expect("paints")
}

/// The colours down column `x` at the middle of rows 1 to 7: the rows
/// after the new one, red and blue in turn.
fn rows_at(pixels: &[u8], x: u32) -> (Vec<[u8; 4]>, Vec<[u8; 4]>) {
    let colours = (1..8).map(|row| pixel(pixels, x, row * 4 + 2)).collect();
    let expected = (1..8)
        .map(|row| if row % 2 == 0 { BLUE } else { RED })
        .collect();
    (colours, expected)
}

/// A clip 4 CSS px wide over a fill twice its size.
const CLIPPED_CSS: &str = ".row { height: 2px; } \
    .clip { width: 4px; height: 2px; overflow: hidden; } \
    .fill { width: 8px; height: 4px; background-color: #f00; } \
    .blue .fill { background-color: #00f; }";

#[test]
fn gpu_rows_with_a_clipped_box_each_show_it_where_the_insert_puts_the_row() {
    // Given / When
    let children = [("div", "clip"), ("div", "fill")];
    let pixels = drawn_after_an_insert(CLIPPED_CSS, &children, |_| ());

    // Then: each row shows its fill inside its clip, nothing past it
    let (colours, expected) = rows_at(&pixels, 2);
    assert_eq!(colours, expected);
    assert_eq!(pixel(&pixels, 12, 6), WHITE);
}

/// A clip over an image twice its size, red or blue by the row.
const CLIPPED_IMAGE_CSS: &str = ".row { height: 2px; } \
    .clip { width: 4px; height: 2px; overflow: hidden; } \
    img { display: block; width: 8px; height: 4px; }";

#[test]
fn gpu_rows_with_a_clipped_image_each_show_it_where_the_insert_puts_the_row() {
    // Given / When: each row's image shows its row's colour
    let children = [("div", "clip"), ("img", "")];
    let pixels = drawn_after_an_insert(CLIPPED_IMAGE_CSS, &children, |doc| {
        register(doc, "red.png", RED);
        register(doc, "blue.png", BLUE);
        for (at, row) in doc.children(doc.body()).into_iter().enumerate() {
            let image = doc.children(doc.children(row)[0])[0];
            doc.set_attribute(
                image,
                "src",
                if at % 2 == 0 { "red.png" } else { "blue.png" },
            );
        }
    });

    // Then
    let (colours, expected) = rows_at(&pixels, 2);
    assert_eq!(colours, expected);
    assert_eq!(pixel(&pixels, 12, 6), WHITE);
}

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

/// A clip 6 CSS px wide and a row tall, over one 4 px wide and two rows
/// tall, over a fill larger than both: only their meet shows.
const NESTED_CSS: &str = ".row { height: 2px; } \
    .outer { width: 6px; height: 2px; overflow: hidden; } \
    .inner { width: 4px; height: 4px; overflow: hidden; } \
    .fill { width: 8px; height: 8px; background-color: #f00; } \
    .blue .fill { background-color: #00f; }";

#[test]
fn gpu_rows_with_a_nested_clip_each_show_its_meet_where_the_insert_puts_the_row() {
    // Given / When
    let children = [("div", "outer"), ("div", "inner"), ("div", "fill")];
    let pixels = drawn_after_an_insert(NESTED_CSS, &children, |_| ());

    // Then: inside both clips, the row's fill; inside the outer clip
    // past the inner one, nothing
    let (colours, expected) = rows_at(&pixels, 2);
    assert_eq!(colours, expected);
    assert_eq!(pixel(&pixels, 10, 6), WHITE);
}
