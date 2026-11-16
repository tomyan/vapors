//! GPU tier: shapes and meshes in a framed lane's rows (option (b) of
//! the lane fold, `docs/plans/splice-layout.md` section 11.2): each row's
//! shape step is batched in the row's own frame, drawn at that frame's
//! shift, so after a row is put in at the top every row's rect (a closed
//! form) and triangle (a mesh) are where the insert puts the row (scale
//! 2).

use crate::support_surface as support;

use support::{BLUE, RED, Rig, document, pixel, update};
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;

/// Rows 4 CSS px tall, each an svg of 4 by 4: 8 device px a row.
const CSS: &str = ".row { height: 4px; } svg { display: block; }";

/// Device pixels per CSS pixel.
const SCALE: f32 = 2.0;

/// A row holding a rect filling the svg's left half in `left` and a
/// triangle filling its right half's top-left corner in `right`, put in
/// the body before `before` (at the end when `None`).
fn add_row(doc: &mut Document, (left, right): (&str, &str), before: Option<NodeId>) {
    let row = doc.create_element("div");
    doc.set_attribute(row, "class", "row");
    doc.insert_before(doc.body(), row, before);
    let svg = doc.create_element("svg");
    let svg_attributes = [
        ("viewBox", "0 0 4 4"),
        ("width", "4"),
        ("height", "4"),
        ("aria-hidden", "true"),
    ];
    for (name, value) in svg_attributes {
        doc.set_attribute(svg, name, value);
    }
    doc.append(row, svg);
    let rect = [("x", "0"), ("y", "0"), ("width", "2"), ("height", "4")];
    let path = [("d", "M2 0 L4 0 L2 4 Z")];
    for (tag, attributes, fill) in [("rect", &rect[..], left), ("path", &path[..], right)] {
        let shape = doc.create_element(tag);
        for (name, value) in attributes.iter().chain(&[("fill", fill)]) {
            doc.set_attribute(shape, name, value);
        }
        doc.append(svg, shape);
    }
}

/// The colours of row `at`: red then blue on even rows, swapped on odd.
fn colours(at: usize) -> (&'static str, &'static str) {
    if at.is_multiple_of(2) {
        ("red", "blue")
    } else {
        ("blue", "red")
    }
}

#[test]
fn gpu_shape_rows_put_after_an_insert_draw_each_at_its_rows_offset() {
    // Given: 40 rows, colours swapping row to row, drawn once
    let mut doc = document(CSS, &[]);
    for at in 0..40 {
        add_row(&mut doc, colours(at), None);
    }
    update(&mut doc, SCALE);
    let mut rig = Rig::new();
    rig.draw(&doc).expect("paints");

    // When: an odd row's colours put in at the top
    let first = doc.children(doc.body())[0];
    add_row(&mut doc, colours(1), Some(first));
    update(&mut doc, SCALE);
    let pixels = rig.draw(&doc).expect("paints");

    // Then: the lane is framed, and each of the four rows in the 32 px
    // target is where the insert puts it: its rect at device x 0 to 4,
    // its triangle's corner at x 4 to 8 near the row's top
    assert!(
        doc.update_counts().lanes_framed > 0,
        "{:?}",
        doc.update_counts()
    );
    let drawn: Vec<_> = (0..4)
        .map(|row| {
            (
                pixel(&pixels, 1, row * 8 + 4),
                pixel(&pixels, 5, row * 8 + 1),
            )
        })
        .collect();
    let expected: Vec<_> = (0..4)
        .map(|row| {
            if row % 2 == 0 {
                (BLUE, RED)
            } else {
                (RED, BLUE)
            }
        })
        .collect();
    assert_eq!(drawn, expected);
}
