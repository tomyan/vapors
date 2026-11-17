//! GPU tier: an image cut by a clip on the clip table (flat-scene F3.1b,
//! `docs/plans/flat-scene-f3.md` item 8): at 1x and 2x an image twice
//! its box's size draws inside the box as it draws unclipped and nothing
//! outside it, the pixels the scissor gave: a clip on whole device
//! pixels (paint snaps it until F4a) covers each pixel centre fully or
//! not at all.

use crate::support_surface as support;

use support::{Rig, SIZE, document, update};
use vapors_dom::Dom;
use vapors_scene::{RgbaImage, Size};

/// A 5 by 3 CSS px box over a red image of 10 by 6, clipping it or not.
fn drawn(overflow: &str, scale: f32) -> Vec<u8> {
    let css = format!(
        ".cut {{ overflow: {overflow}; width: 5px; height: 3px; }} \
         img {{ display: block; width: 10px; height: 6px; }}"
    );
    let mut doc = document(&css, &[(None, "div", "cut"), (Some(0), "img", "")]);
    let image = RgbaImage {
        size: Size {
            width: 1,
            height: 1,
        },
        rgba: vec![255, 0, 0, 255],
    };
    doc.register_image("red.png", image).expect("registers");
    let img = doc.children(doc.children(doc.body())[0])[0];
    doc.set_attribute(img, "src", "red.png");
    update(&mut doc, scale);
    Rig::new().draw(&doc).expect("paints")
}

#[test]
fn gpu_an_image_at_a_clip_edge_draws_inside_as_unclipped_and_nothing_outside() {
    for scale in [1.0_f32, 2.0] {
        // Given: the same image clipped by its box, and not
        let (clipped, whole) = (drawn("hidden", scale), drawn("visible", scale));

        // Then: inside the box the same pixels, outside the page's white
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // a few device px
        let (right, bottom) = ((5.0 * scale) as usize, (3.0 * scale) as usize);
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
            "the image must reach past the clip at {scale}x"
        );
    }
}
