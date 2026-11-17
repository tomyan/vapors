//! GPU tier: a page painted at 4 samples against the same page at 1,
//! within bounds measured first (`docs/plans/flat-scene-f7.md`, Q7).
//! Analytic 2D coverage is the same at every sample; a pixel differs only
//! where a triangle's edge crosses it, so shapes off the pixel grid
//! differ at their edges (the demos, identical, are in `gpu_demos.rs`);
//! a tooltip at 4 samples draws as at 1; and a tooltip on a view after
//! another view's frame is refused, not drawn over that view's: the two
//! share the size's 4x texture, which holds the other's frame.

use vapors_renderer::Samples;

use crate::support_surface as support;

use support::{Rig, SIZE, bare_document, document, update};
use vapors_dom::Dom;

/// The largest channel difference and the differing pixels of the
/// shapes page at 4 samples against 1, as measured on 2026-11-17 on ANV
/// (Intel UHD 630) and on llvmpipe: (29, 51) on both.
pub(crate) const SHAPES_BOUND: (u8, usize) = (29, 51);

/// The largest channel difference, and how many pixels differ at all,
/// between `one` and `four`.
pub(crate) fn difference(one: &[u8], four: &[u8]) -> (u8, usize) {
    let pixels = one.chunks(4).zip(four.chunks(4));
    let most = one.iter().zip(four).map(|(a, b)| a.abs_diff(*b)).max();
    let differ = pixels.filter(|(a, b)| a != b).count();
    (most.unwrap_or(0), differ)
}

/// An svg of a triangle (a mesh) and a circle (a closed form), neither
/// edge on the pixel grid, updated.
pub(crate) fn shapes() -> vapors_elements::Document {
    let mut doc = document("svg { display: block; }", &[]);
    let svg = doc.create_element("svg");
    let size = [
        ("viewBox", "0 0 32 32"),
        ("width", "32"),
        ("height", "32"),
        ("aria-hidden", "true"),
    ];
    for (name, value) in size {
        doc.set_attribute(svg, name, value);
    }
    doc.append(doc.body(), svg);
    let triangle = [("d", "M3 2 L29 9 L7 30 Z"), ("fill", "red")];
    let circle = [("cx", "20"), ("cy", "20"), ("r", "7.3"), ("fill", "blue")];
    for (tag, attributes) in [("path", &triangle[..]), ("circle", &circle[..])] {
        let shape = doc.create_element(tag);
        for &(name, value) in attributes {
            doc.set_attribute(shape, name, value);
        }
        doc.append(svg, shape);
    }
    update(&mut doc, 1.0);
    doc
}

#[test]
fn gpu_shapes_off_the_pixel_grid_at_4_samples_differ_from_1_within_the_measured_bound() {
    // When: each drawn by its own painter (a painter takes a document's
    // shapes as they change)
    let one = Rig::with_samples(Samples::One)
        .draw(&shapes())
        .expect("paints");
    let four = Rig::with_samples(Samples::Four)
        .draw(&shapes())
        .expect("paints");

    // Then: some pixel differs (the pass is multisampled), within the
    // bound
    let (most, differ) = difference(&one, &four);
    assert!(
        differ > 0,
        "no pixel differs: the 4-sample painter drew at 1 sample"
    );
    assert!(
        most <= SHAPES_BOUND.0 && differ <= SHAPES_BOUND.1,
        "({most}, {differ}) is past the measured {SHAPES_BOUND:?}"
    );
}

#[test]
fn gpu_draw_over_at_4_samples_is_refused_naming_it() {
    // Given
    let mut doc = bare_document("", &[]);
    update(&mut doc, 1.0);

    // When
    let refused = Rig::with_samples(Samples::Four)
        .draw_over(&doc)
        .expect_err("refused");

    // Then
    assert!(
        refused.to_string().starts_with(
            "draw_over at 4 samples: the painter draws into its own 4x texture, which holds \
             nothing of what the target holds"
        ),
        "{refused}"
    );
}

/// A page styled for tooltips (`support/tooltip.rs`), updated.
fn tooltip_page() -> vapors_elements::Document {
    let mut doc = document(crate::support_tooltip::TOOLTIP, &[]);
    update(&mut doc, 1.0);
    doc
}

/// A pointer tooltip of one dot over `doc`'s body.
fn tip(doc: &vapors_elements::Document) -> vapors_app::Tooltip {
    vapors_app::Tooltip {
        node: doc.body(),
        text: ".".to_owned(),
        trigger: vapors_app::TooltipTrigger::Pointer,
    }
}

#[test]
fn gpu_a_tooltip_at_4_samples_draws_as_at_1() {
    // When: the page, then the tooltip over it, at each count
    let at = |samples| {
        let mut doc = tooltip_page();
        let tooltip = tip(&doc);
        Rig::with_samples(samples)
            .draw_tooltip(&mut doc, &tooltip, Some([2.0, 2.0]))
            .expect("draws")
    };
    let (one, four) = (at(Samples::One), at(Samples::Four));

    // Then: drawn, and the same pixels (quads and text on whole pixels)
    let green = crate::support_tooltip::GREEN;
    assert!(
        crate::support_tooltip::bounds(&four, green).is_some(),
        "no tooltip drawn"
    );
    assert_eq!(difference(&one, &four), (0, 0));
}

/// Draws into two views of one size at 4 samples, the first in encoder
/// `encoders.0` and the second in `encoders.1` (0 or 1 of two), then a
/// tooltip on view `tooltip_on` in encoder `encoders.2`.
fn tooltip_after_two_draws(encoders: (usize, usize, usize), tooltip_on: usize) {
    let mut doc = tooltip_page();
    let tooltip = tip(&doc);
    let mut rig = Rig::with_samples(Samples::Four);
    let gpu = &rig.gpu;
    let made = || {
        let colour = vapors_scene::Color::default();
        let texture = vapors_renderer::clear(&gpu.device, &gpu.queue, SIZE, SIZE, colour);
        texture.create_view(&wgpu::TextureViewDescriptor::default())
    };
    let views = [made(), made()];
    let both = [0, 1].map(|_| vapors_renderer::FrameEncoder::new(&gpu.device));
    let on = |view: usize, encoder: usize| vapors_renderer::Target {
        device: &gpu.device,
        queue: &gpu.queue,
        encoder: &both[encoder],
        view: &views[view],
        multisampled: None,
        viewport: vapors_scene::Size {
            width: SIZE,
            height: SIZE,
        },
        scissor: None,
    };
    let painted = doc.painted().expect("updated");
    rig.painter
        .draw(&on(0, encoders.0), painted)
        .expect("draws");
    rig.painter
        .draw(&on(1, encoders.1), painted)
        .expect("draws");
    let target = on(tooltip_on, encoders.2);
    let _ = rig
        .painter
        .draw_tooltip(&target, &mut doc, &tooltip, Some([2.0, 2.0]));
}

#[test]
#[should_panic(expected = "its resolve would put that frame on this one")]
fn gpu_a_tooltip_on_a_view_after_another_views_frame_at_4_samples_is_refused() {
    // A and B in one encoder, then a tooltip on A, whose frame B replaced
    tooltip_after_two_draws((0, 0, 0), 0);
}

#[test]
#[should_panic(expected = "which this pipeline drew for, is still open")]
fn gpu_a_draw_into_another_encoder_while_the_first_is_open_is_refused_before_any_tooltip() {
    // A in one encoder, B in another while the first is open: the
    // pipelines refuse B's draw (encoder_turn.rs), so a tooltip on B in
    // A's encoder, which would resolve A's frame onto B, cannot follow
    tooltip_after_two_draws((0, 1, 0), 1);
}

#[path = "samples/app_frames.rs"]
mod app_frames;
