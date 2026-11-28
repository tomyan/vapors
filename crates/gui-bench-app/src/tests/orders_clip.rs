//! The orders card clips its header and rows to its 12px corners (F3.2c,
//! `docs/plans/flat-scene-f3.md`): `.orders` has `overflow: hidden`, so
//! the header no longer rounds its own top corners, and a row's hover
//! background is cut by the card's bottom corners, as zgui's is. With 40
//! orders the card's rows are a lane, framed, their frame's clip the
//! card's rounded one (`docs/plans/frame-clips.md`, slices 2 and 3).
use super::support::*;
use vapors_scene::{Clip, Item};

/// The rows the card holds: past the 32 a lane frames.
const ORDERS: usize = 40;

/// The first element of `class`'s quad painted with `background`: its
/// radii and its clip.
// Exact: the quad and the border box are read from one layout at scale 1.
#[allow(clippy::float_cmp)]
fn quad_of(
    mounted: &crate::app::Mounted,
    class: &str,
    background: u32,
) -> ([f32; 4], Option<Clip>) {
    let doc = mounted.document.borrow();
    let [x, y, ..] = rect(&doc, by_class(&doc, class)[0]);
    let painted = doc.painted().expect("laid out");
    let mut found = painted
        .scene()
        .items()
        .filter_map(|(item, clip)| match item {
            Item::Quad(quad)
                if quad.background == rgb(background)
                    && [quad.rect.left, quad.rect.top] == [x, y] =>
            {
                Some((quad.radii, clip))
            }
            _ => None,
        });
    found
        .next()
        .expect("a quad of that background at the element")
}

#[test]
fn the_header_is_square_under_the_cards_rounded_clip() {
    // Given
    let mounted = with_rows(ORDERS);
    let card = {
        let doc = mounted.document.borrow();
        rect(&doc, by_class(&doc, "orders")[0])
    };

    // When
    let (radii, clip) = quad_of(&mounted, "orders-head", 0x00f9_fafb);

    // Then: no radius of its own; the card's 12px padding box clips it
    assert_eq!(radii, [0.0; 4]);
    let clip = clip.expect("the header is clipped");
    assert_eq!(clip.radii(), [12.0; 4]);
    let [x, y, width, height] = card;
    assert_eq!(
        clip.rect(),
        vapors_scene::Rect::new(x, y, x + width, y + height)
    );
}

#[test]
fn a_hovered_rows_background_is_under_the_cards_rounded_clip() {
    // Given: the pointer on the first row
    let mounted = with_rows(ORDERS);
    let row = {
        let doc = mounted.document.borrow();
        rect(&doc, by_class(&doc, "order-row")[0])
    };
    let [x, y, width, height] = row;
    let centre = [x + width / 2.0, y + height / 2.0];
    vapors_elements::Document::pointer_moved(&mounted.document, centre);
    refresh(&mounted);

    // When
    let (_, clip) = quad_of(&mounted, "order-row", 0x00ee_f2ff);

    // Then
    assert_eq!(clip.map(|clip| clip.radii()), Some([12.0; 4]));
}

#[test]
fn the_cards_rows_are_framed_under_its_rounded_clip() {
    // When: 40 orders; 31, whose card holds 32 children (a header and
    // the rows), the fewest a lane frames; and 30, one child fewer
    let framed = |orders: usize| {
        let mounted = with_rows(orders);
        let counts = mounted.document.borrow().update_counts();
        (counts.lanes_framed, counts.lanes_unframed)
    };

    // Then: the card's lane is the one framed, and none declined
    assert_eq!(framed(ORDERS), (1, 0));
    assert_eq!(framed(31), (1, 0));
    assert_eq!(framed(30), (0, 0));
}
