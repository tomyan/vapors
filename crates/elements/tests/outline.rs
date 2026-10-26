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
fn an_auto_dashed_dotted_or_double_outline_is_refused() {
    // Given
    for style in ["auto", "dashed", "dotted", "double"] {
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

#[test]
fn a_negative_offset_never_inverts_the_ring_or_its_corners() {
    // Given: rounded boxes whose outline is pulled 8px inside: at 10x10
    // the ring would turn inside out; at 40x40 it stays upright but its
    // 3px corners would go to -3
    let mut rings = Vec::new();
    for size in [10, 40] {
        rings.extend(items(&format!("body {{ margin: 0; }} .box {{ width: {size}px; height: {size}px; border-radius: 3px; outline: 2px solid red; outline-offset: -8px; }}")));
    }
    assert!(!rings.is_empty(), "the 40px box draws its ring");

    // Then: either no ring, or one with an upright rect and corners of zero or more
    for item in &rings {
        if let Item::Quad(ring) = item
            && ring.border_colors[0].to_rgba8() == [255, 0, 0, 255]
        {
            assert!(
                ring.rect.left <= ring.rect.right && ring.rect.top <= ring.rect.bottom,
                "{ring:?}"
            );
            assert!(ring.radii.iter().all(|r| *r >= 0.0), "{ring:?}");
        }
    }
}

#[test]
fn an_overlay_above_paints_over_an_outline_below() {
    // Given: an outlined box, and a later positioned box with z-index 1
    // over it
    let items = items(
        "body { margin: 0; } .box { width: 40px; height: 40px; outline: 4px solid red; } .after { position: absolute; left: 0; top: 0; width: 60px; height: 60px; z-index: 1; background-color: blue; }",
    );

    // Then: the ring is painted before the overlay, as Chromium paints an
    // outline last within its own stacking context, not the page
    let at = |rgba: [u8; 4], border: bool| {
        items.iter().position(|item| match item {
            Item::Quad(q) if border => q.border_colors[0].to_rgba8() == rgba && q.border[0] > 0.0,
            Item::Quad(q) => q.background.to_rgba8() == rgba,
            _ => false,
        })
    };
    let (ring, overlay) = (at([255, 0, 0, 255], true), at([0, 0, 255, 255], false));
    assert!(
        ring.is_some() && overlay.is_some() && ring < overlay,
        "{items:?}"
    );
}

#[test]
fn an_overflow_hidden_ancestor_clips_a_childs_outline_but_not_its_own() {
    // Given: a clipping parent with an outline, holding an outlined child
    let mut doc = document();
    let body = doc.body();
    let parent = element(&mut doc, body, "div", "parent");
    element(&mut doc, parent, "div", "child");
    doc.add_stylesheet(
        "app.css",
        "body { margin: 20px; } .parent { width: 50px; height: 50px; overflow: hidden; outline: 2px solid red; } .child { width: 20px; height: 20px; outline: 3px solid blue; }",
    )
    .expect("parses");

    // When
    let painted = doc.update(viewport(200.0, 100.0)).expect("updates");
    let scene = painted.scene();

    // Then: the child's ring is under the parent's clip; the parent's is not
    let clip_of = |rgba: [u8; 4]| {
        scene
            .items()
            .iter()
            .enumerate()
            .find_map(|(i, item)| match item {
                Item::Quad(q) if q.border_colors[0].to_rgba8() == rgba && q.border[0] > 0.0 => {
                    Some(scene.clips()[i])
                }
                _ => None,
            })
    };
    assert!(
        clip_of([0, 0, 255, 255]).flatten().is_some(),
        "the child's ring is clipped"
    );
    assert!(
        clip_of([255, 0, 0, 255]).flatten().is_none(),
        "the parent's own ring is not"
    );
}
