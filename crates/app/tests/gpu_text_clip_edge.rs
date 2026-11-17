//! GPU tier: text cut by a clip on the clip table (flat-scene F3.1b,
//! `docs/plans/flat-scene-f3.md` item 8): at a clip edge through its
//! glyphs, at 1x and 2x, the text draws inside the clip exactly as it
//! draws unclipped, and nothing outside it, the pixels the scissor gave
//! when text was cut by it: a whole-pixel clip's coverage is 0 or 1 at
//! each pixel centre.

use crate::support_surface as support;

use support::{Rig, SIZE, document, update};
use vapors_dom::Dom;

/// A 5 CSS px wide box whose text runs past its right edge, clipping it
/// or not.
fn drawn(overflow: &str, scale: f32) -> Vec<u8> {
    let css =
        format!(".cut {{ overflow: {overflow}; width: 5px; height: 12px; font-size: 12px; }}");
    let mut doc = document(&css, &[(None, "div", "cut")]);
    let cut = doc.children(doc.body())[0];
    let text = doc.create_text("Wg");
    doc.append(cut, text);
    update(&mut doc, scale);
    Rig::new().draw(&doc).expect("paints")
}

#[test]
fn gpu_text_at_a_clip_edge_draws_inside_as_unclipped_and_nothing_outside() {
    for scale in [1.0_f32, 2.0] {
        // Given: the same text clipped by its box, and not
        let (clipped, whole) = (drawn("hidden", scale), drawn("visible", scale));

        // Then: inside the box the same pixels, outside the page's white
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // a few device px
        let (right, bottom) = ((5.0 * scale) as usize, (12.0 * scale) as usize);
        let mut inked_outside_unclipped = false;
        for (at, (clipped, whole)) in clipped.chunks(4).zip(whole.chunks(4)).enumerate() {
            let (x, y) = (at % SIZE as usize, at / SIZE as usize);
            if x < right && y < bottom {
                assert_eq!(clipped, whole, "pixel {x}, {y} at {scale}x");
            } else {
                assert_eq!(
                    clipped, [255; 4],
                    "pixel {x}, {y} at {scale}x, outside the clip"
                );
                inked_outside_unclipped |= whole != [255; 4];
            }
        }
        assert!(
            inked_outside_unclipped,
            "the text must run past the clip at {scale}x"
        );
    }
}
