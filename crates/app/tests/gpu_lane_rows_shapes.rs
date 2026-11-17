//! GPU tier: shapes and meshes in a framed lane's rows
//! (`docs/plans/splice-layout.md` section 11.2; `flat-scene-f2.md`,
//! F2.3b): each item names its row, placed by its row's entry in the
//! transform table, so after a row is put in at the top every row's rect
//! (a closed form) and triangle (a mesh) are where the insert puts the
//! row (scale 2); and rows each holding a rect are one shape draw, drawn
//! as a fresh painter draws a page built with the insert already made.

use crate::support_surface as support;

use support::{BLUE, RED, Rig, document, pixel, update};
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;

/// Rows 4 CSS px tall, each an svg of 4 by 4: 8 device px a row.
const CSS: &str = ".row { height: 4px; } svg { display: block; }";

/// Device pixels per CSS pixel.
const SCALE: f32 = 2.0;

/// A row holding a rect filling the svg's left half in `left` and, when
/// `right` is given, a triangle filling its right half's top-left corner
/// in it, put in the body before `before` (at the end when `None`).
fn add_row(doc: &mut Document, (left, right): (&str, Option<&str>), before: Option<NodeId>) {
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
    let shapes = [("rect", &rect[..], Some(left)), ("path", &path[..], right)];
    for (tag, attributes, fill) in shapes {
        let Some(fill) = fill else { continue };
        let shape = doc.create_element(tag);
        for (name, value) in attributes.iter().chain(&[("fill", fill)]) {
            doc.set_attribute(shape, name, value);
        }
        doc.append(svg, shape);
    }
}

/// The colours of row `at`: red then blue on even rows, swapped on odd.
fn colours(at: usize) -> (&'static str, Option<&'static str>) {
    if at.is_multiple_of(2) {
        ("red", Some("blue"))
    } else {
        ("blue", Some("red"))
    }
}

/// 40 rows of `colours`, updated.
fn rows_of(colours: fn(usize) -> (&'static str, Option<&'static str>)) -> Document {
    let mut doc = document(CSS, &[]);
    for at in 0..40 {
        add_row(&mut doc, colours(at), None);
    }
    update(&mut doc, SCALE);
    doc
}

#[test]
fn gpu_shape_rows_put_after_an_insert_draw_each_at_its_rows_offset() {
    // Given: 40 rows, colours swapping row to row, drawn once
    let mut doc = rows_of(colours);
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

#[test]
fn gpu_a_rect_in_each_row_put_after_an_insert_is_one_shape_draw_as_a_fresh_painter_draws_it() {
    // Given: 40 rows, each a rect, drawn once
    let mut doc = rows_of(|at| (colours(at).0, None));
    let mut rig = Rig::new();
    rig.draw(&doc).expect("paints");

    // When: a row put in at the top
    let first = doc.children(doc.body())[0];
    add_row(&mut doc, ("blue", None), Some(first));
    update(&mut doc, SCALE);
    let kept = rig.draw(&doc).expect("paints");

    // Then: the rects of the rows in view are one draw, and the pixels
    // are a fresh painter's of a page built with the row already first (a
    // painter takes a document's shapes as they change, so a fresh one
    // draws a fresh document)
    let shapes = rig.painter.frame_counts().shapes;
    assert_eq!(shapes.draw_calls, 1, "{shapes:?}");
    let mut built = document(CSS, &[]);
    add_row(&mut built, ("blue", None), None);
    for at in 0..40 {
        add_row(&mut built, (colours(at).0, None), None);
    }
    update(&mut built, SCALE);
    let fresh = Rig::new().draw(&built).expect("paints");
    assert!(kept.contains(&255), "the rects drew nothing");
    assert!(kept == fresh, "the rects differ from a fresh draw");
}
