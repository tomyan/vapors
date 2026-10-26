//! Outlines (3-E2b): drawn outside the border box at `outline-offset`,
//! not laid out, after everything else in the scene, and refused where
//! vapors cannot draw them.

mod support;

use support::{document, element, refusals, viewport};
use vapors_scene::{Color, Item};

fn items(sheet: &str) -> Vec<Item> {
    let mut doc = document();
    let body = doc.body();
    element(&mut doc, body, "div", "box");
    element(&mut doc, body, "div", "after");
    doc.add_stylesheet("app.css", sheet).expect("parses");
    doc.update(viewport(200.0, 100.0))
        .expect("updates")
        .scene()
        .items()
        .to_vec()
}

#[test]
fn an_outline_rings_the_border_box_at_its_offset_and_is_drawn_last() {
    // Given: a 10x10 box at (0, 0) with a 2px red outline 1px out, and a
    // later sibling with a blue background
    let sheet = "body { margin: 0; } .box { width: 10px; height: 10px; outline: 2px solid red; outline-offset: 1px; } .after { height: 10px; background-color: blue; }";

    // When
    let items = items(sheet);

    // Then: the last item is the ring: 3px out on each side, 2px wide, red
    let Some(Item::Quad(ring)) = items.last() else {
        panic!("a quad last: {items:?}")
    };
    assert_eq!(
        (
            ring.rect.left,
            ring.rect.top,
            ring.rect.right,
            ring.rect.bottom
        ),
        (-3.0, -3.0, 13.0, 13.0)
    );
    assert_eq!(ring.border, [2.0; 4]);
    assert_eq!(ring.border_colors[0].to_rgba8(), [255, 0, 0, 255]);
    assert_eq!(ring.background, Color::default());
}

#[test]
fn an_outline_takes_no_room_in_layout() {
    // Given: the same box with a 5px outline, and without
    let with = items(
        "body { margin: 0; } .box { height: 10px; outline: 5px solid red; } .after { height: 10px; background-color: blue; }",
    );
    let without = items(
        "body { margin: 0; } .box { height: 10px; } .after { height: 10px; background-color: blue; }",
    );

    // Then: the sibling's quad is in the same place
    let blue = |items: &[Item]| {
        items.iter().find_map(|item| match item {
            Item::Quad(q) if q.background.to_rgba8() == [0, 0, 255, 255] => Some(q.rect.top),
            _ => None,
        })
    };
    assert_eq!(blue(&with), blue(&without));
}

#[test]
fn no_outline_by_default_and_none_draws_nothing() {
    // Then: only the sibling's background
    assert_eq!(
        items("body { margin: 0; } .after { height: 10px; background-color: blue; }").len(),
        1
    );
    assert_eq!(items("body { margin: 0; } .box { outline: 3px none red; } .after { height: 10px; background-color: blue; }").len(), 1);
}

#[test]
fn an_auto_or_dashed_outline_is_refused() {
    // Given
    for style in ["auto", "dashed"] {
        let mut doc = document();
        let body = doc.body();
        element(&mut doc, body, "div", "box");
        doc.add_stylesheet("app.css", &format!(".box {{ outline: 2px {style} red; }}"))
            .expect("parses");

        // When
        let errors = refusals(&mut doc);

        // Then
        assert!(
            errors
                .iter()
                .any(|e| e.contains("outline-style") && e.contains(style)),
            "{errors:?}"
        );
    }
}
