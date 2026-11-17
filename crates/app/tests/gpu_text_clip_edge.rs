//! GPU tier: text cut by a clip on the clip table (flat-scene F3.1b,
//! `docs/plans/flat-scene-f3.md` item 8): at a clip edge through its
//! glyphs, at 1x and 2x, the text draws inside the clip exactly as it
//! draws unclipped, and nothing outside it, the pixels the scissor gave
//! when text was cut by it: a whole-pixel clip's coverage is 0 or 1 at
//! each pixel centre. At 1.5x the 5 CSS px edge falls at 7.5 device px;
//! paint snaps it to a whole pixel (`painter_clips.rs` `assert_whole`
//! refuses any other until F4a), so there too the cut is a whole column
//! with no partial one.

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

#[test]
fn gpu_text_at_a_fractional_scale_is_cut_at_a_whole_device_column() {
    // Given: at 1.5x the box's right edge is 7.5 device px
    let (clipped, whole) = (drawn("hidden", 1.5), drawn("visible", 1.5));

    // When: the first column where the clipped text differs from it unclipped
    let size = SIZE as usize;
    let column = |pixels: &[u8], x: usize| -> Vec<u8> {
        (0..size)
            .flat_map(|y| pixels[(y * size + x) * 4..][..4].to_vec())
            .collect()
    };
    let cut = (0..size)
        .find(|&x| column(&clipped, x) != column(&whole, x))
        .expect("the clip cuts");

    // Then: the edge snapped to 7 or 8, every column from it blank
    assert!(cut == 7 || cut == 8, "cut at column {cut}");
    for x in cut..size {
        assert!(
            column(&clipped, x).iter().all(|&byte| byte == 255),
            "column {x} past the cut"
        );
    }
}

#[test]
fn gpu_kept_text_whose_clip_changed_is_built_again() {
    // Given: the clipped text drawn and kept
    let css = ".cut { overflow: hidden; width: 5px; height: 12px; font-size: 12px; } \
        .short { height: 6px; }";
    let mut doc = document(css, &[(None, "div", "cut")]);
    let cut = doc.children(doc.body())[0];
    let text = doc.create_text("Wg");
    doc.append(cut, text);
    update(&mut doc, 2.0);
    let mut rig = Rig::new();
    rig.draw(&doc).expect("paints");

    // When: the box grows shorter, so its clip changes and the text, the
    // batch's item, and so its key, do not
    doc.set_attribute(cut, "class", "cut short");
    update(&mut doc, 2.0);
    let pixels = rig.draw(&doc).expect("paints");

    // Then: the text's bytes built again (written this frame), naming the
    // new entry, and drawn as afresh
    let counts = rig.painter.frame_counts();
    assert!(counts.texts.buffer_bytes > 0, "{counts:?}");
    assert!(
        pixels == Rig::new().draw(&doc).expect("paints"),
        "drawn other than afresh"
    );
}
