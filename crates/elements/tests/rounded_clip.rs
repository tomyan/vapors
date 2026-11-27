//! Rounded clips (flat-scene F3.2b, `docs/plans/flat-scene-f3.md`): a
//! box with `overflow: hidden` and a `border-radius` clips what is in it
//! to its padding box with the padding box's radii (its radii less its
//! border). A clip meets a rounded clip only where one holds the other;
//! an overlap, an img and a frame under one are refused. A lane whose
//! own box is rounded is not framed.

use crate::support;

use support::lane_page::header_list;
use support::narrow_list::ROWS;
use support::{document, element, refusals, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, LaneDecline, Viewport};
use vapors_scene::{Clip, Color, Item, Rect, RgbaImage, Size};

/// A 100 x 60 card, a 1px border and 12px corners, clipping; `.fill`
/// is a tall blue block.
const CARD: &str = "body { margin: 0; } \
    .card { overflow: hidden; width: 100px; height: 60px; border: 1px solid black; border-radius: 12px; } \
    .fill { height: 200px; background-color: blue; }";

const BLUE: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 1.0,
    a: 1.0,
};

fn at(scale: f32) -> Viewport {
    Viewport {
        scale,
        ..viewport(400.0, 300.0)
    }
}

/// A card holding `inner` (a class, or empty for none) holding a fill.
fn card(css: &str, inner: &str) -> (Document, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let card = element(&mut doc, body, "div", "card");
    let parent = if inner.is_empty() {
        card
    } else {
        element(&mut doc, card, "div", inner)
    };
    element(&mut doc, parent, "div", "fill");
    doc.add_stylesheet("page.css", &format!("{CARD} {css}"))
        .expect("parses");
    (doc, card)
}

/// The clip of the blue fill's quad.
fn fill_clip(doc: &Document) -> Option<Clip> {
    let painted = doc.painted().expect("painted");
    let mut fills = painted
        .scene()
        .items()
        .filter_map(|(item, clip)| match item {
            Item::Quad(quad) if quad.background == BLUE => Some(clip),
            _ => None,
        });
    fills.next().expect("the fill is painted")
}

#[test]
fn what_a_rounded_hidden_box_holds_is_clipped_to_its_padding_radii() {
    for scale in [1.0, 2.0] {
        // Given
        let (mut doc, _) = card("", "");

        // When
        doc.update(at(scale)).expect("updates");

        // Then: the padding box, the radii less the 1px border
        let padding = Rect::new(scale, scale, 101.0 * scale, 61.0 * scale);
        let want = Clip::rounded(padding, [11.0 * scale; 4]);
        assert_eq!(fill_clip(&doc), Some(want), "at {scale}x");
    }
}

#[test]
fn a_clipping_box_clear_of_the_corners_gives_its_content_a_square_clip() {
    // Given: a clipping box 20px down the card, 20px tall
    let css = ".inner { overflow: hidden; margin-top: 20px; height: 20px; }";
    let (mut doc, _) = card(css, "inner");

    // When
    doc.update(at(1.0)).expect("updates");

    // Then
    let want = Clip::square(Rect::new(1.0, 21.0, 101.0, 41.0));
    assert_eq!(fill_clip(&doc), Some(want));
}

#[test]
fn a_clipping_box_across_a_rounded_corner_is_refused() {
    // Given: a clipping box over the card's top corners
    let css = ".inner { overflow: hidden; height: 20px; }";
    let (mut doc, _) = card(css, "inner");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert!(errors.iter().any(|e| e.contains("overlap")), "{errors:?}");
}

#[test]
fn a_scroll_container_holding_a_rounded_clip_is_refused() {
    // Given: a scroller larger than the card's padding box, around it
    let css = ".inner { overflow: auto; margin: -10px; height: 100px; }";
    let (mut doc, _) = card(css, "inner");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert!(errors.iter().any(|e| e.contains("scrolls")), "{errors:?}");
}

#[test]
fn an_img_under_a_rounded_clip_is_refused() {
    // Given
    let (mut doc, card) = card("img { display: block; }", "");
    let image = RgbaImage {
        size: Size {
            width: 4,
            height: 4,
        },
        rgba: [0, 0, 0, 255].repeat(16),
    };
    doc.register_image("a.png", image).expect("registers");
    let img = element(&mut doc, card, "img", "");
    doc.set_attribute(img, "src", "a.png");

    // When
    let errors = refusals(&mut doc);

    // Then
    assert!(
        errors
            .iter()
            .any(|e| e.contains("is an img under a rounded clip")),
        "{errors:?}"
    );
}

#[test]
fn a_radius_change_alone_paints_what_the_box_clips_again() {
    // Given
    let (mut doc, card) = card(".rounder { border-radius: 21px; }", "");
    doc.update(at(1.0)).expect("updates");

    // When
    doc.set_attribute(card, "class", "card rounder");
    doc.update(at(1.0)).expect("updates");

    // Then
    let want = Clip::rounded(Rect::new(1.0, 1.0, 101.0, 61.0), [20.0; 4]);
    assert_eq!(fill_clip(&doc), Some(want));
}

#[test]
fn a_lane_whose_own_box_clips_to_rounded_corners_is_not_framed() {
    // Given: 100 rows in a list that clips them to rounded corners
    let css = format!(
        "body {{ margin: 0; }} .head {{ height: 10px; }} {ROWS} .list {{ overflow: hidden; border-radius: 8px; }}"
    );
    let page = header_list(100, &css, at(1.0));

    // When
    let counts = page.edited(at(1.0), |page| _ = page.insert_at(50));

    // Then
    assert_eq!(
        counts.lane_unframed,
        Some(LaneDecline::LaneFrameRoundedClip),
        "{counts:?}"
    );
}

#[test]
fn a_radius_change_alone_paints_what_the_box_clips_deeper_down_again() {
    // Given: the fill inside a plain div inside the card
    let (mut doc, card) = card("", "plain");
    doc.update(at(1.0)).expect("updates");

    // When: the card's own style alone changes
    doc.set_attribute(card, "style", "border-radius: 21px");
    doc.update(at(1.0)).expect("updates");

    // Then
    let want = Clip::rounded(Rect::new(1.0, 1.0, 101.0, 61.0), [20.0; 4]);
    assert_eq!(fill_clip(&doc), Some(want));
}
