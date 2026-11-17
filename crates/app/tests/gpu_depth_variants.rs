//! GPU tier: every 2D pipeline in its with-depth variant draws what it
//! draws without (flat-scene F8a): a page of boxes with a shadow, text,
//! a mesh, a closed form and a surface's composited image, drawn into a
//! pass with the frame's depth attachment, is the same pixels as without
//! it, at 1 and at 4 samples. 2D draws neither read nor write depth.

use crate::support_surface as support;

use crate::gpu_samples::shapes;
use support::{Rig, Solid};
use vapors_dom::Dom;
use vapors_renderer::Samples;
use vapors_scene::Size;

/// Large enough for the whole page: the shapes' 32 px, the card and the
/// surface below them.
const AREA: Size = Size {
    width: 96,
    height: 96,
};

/// The shapes page with a box (a quad and a shadow) holding text, and a
/// surface: every 2D pipeline the painter has draws in it.
fn every_kind() -> vapors_elements::Document {
    let mut doc = shapes();
    let css = ".card { width: 40px; height: 20px; border-radius: 4px; background-color: #8ab4f8; \
               box-shadow: 2px 2px 3px #000000; font-size: 12px; } \
               .view { display: block; width: 16px; height: 16px; } svg { display: block; }";
    doc.add_stylesheet("depth.css", css).expect("parses");
    let card = doc.create_element("div");
    doc.set_attribute(card, "class", "card");
    let text = doc.create_text("Depth");
    doc.append(card, text);
    doc.append(doc.body(), card);
    let surface = doc.create_element("gpu-surface");
    doc.set_attribute(surface, "class", "view");
    doc.set_attribute(surface, "name", "view");
    doc.append(doc.body(), surface);
    support::update(&mut doc, 1.0);
    doc
}

#[test]
fn gpu_a_page_drawn_with_the_frames_depth_attachment_is_the_same_pixels_as_without() {
    for samples in [Samples::One, Samples::Four] {
        // Given: the page drawn by two painters, each with a surface and
        // its own copy (a painter takes a document's shapes as its own)
        let (page, copy) = (every_kind(), every_kind());
        let gpu = vapors_gpu_harness::gpu();
        let rig = || {
            let mut rig = Rig::with_samples_on(gpu.clone(), samples);
            let surface = Box::new(Solid::new([0, 200, 0, 255]).0);
            rig.painter.mount("view", surface).expect("mounts");
            rig
        };
        let (mut flat, mut deep) = (rig(), rig());

        // When
        let without = flat.draw_at(&page, AREA).expect("draws");
        let with = deep.draw_with_depth(&copy, (AREA, samples)).expect("draws");

        // Then: every kind drew, in view, and the same pixels both ways
        assert_every_kind_in_view(&with);
        assert_eq!(without.len(), with.len());
        assert!(
            without == with,
            "{samples:?}: the depth variants drew otherwise"
        );
    }
}

/// The pixel at device `(x, y)` of the `AREA` target's bytes.
fn at(rgba: &[u8], x: u32, y: u32) -> [u8; 4] {
    let i = ((y * AREA.width + x) * 4) as usize;
    rgba[i..i + 4].try_into().expect("four bytes")
}

/// The mesh (red), the closed form (blue), the surface's image (green),
/// the card, its shadow right of the card and its text inside it.
fn assert_every_kind_in_view(rgba: &[u8]) {
    let count = |colour: [u8; 4]| rgba.chunks(4).filter(|p| *p == colour).count();
    assert!(count([255, 0, 0, 255]) > 0, "the mesh in view");
    assert!(count([0, 0, 255, 255]) > 0, "the closed form in view");
    assert_eq!(
        count([0, 200, 0, 255]),
        16 * 16,
        "the surface's image in view"
    );
    assert!(count([0x8a, 0xb4, 0xf8, 255]) > 0, "the card in view");
    let [r, g, b, _] = at(rgba, 40, 44);
    assert!(
        r == g && g == b && r < 0x80,
        "the shadow right of the card: {:?}",
        [r, g, b]
    );
    let dark = (2..38).flat_map(|x| (34..50).map(move |y| (x, y)));
    let inked = dark
        .filter(|&(x, y)| at(rgba, x, y)[..3].iter().all(|&c| c < 0x40))
        .count();
    assert!(inked > 0, "the card's text in view");
}

#[test]
fn gpu_a_pass_after_a_clear_outside_the_encoder_clears_its_depth() {
    // Given: the rig clears its target before the encoder, so draw_over's
    // pass loads its colour; the depth texture starts at wgpu's zero
    let mut rig = Rig::with_samples(Samples::One);
    rig.painter
        .mount("view", Box::new(Solid::new([0, 200, 0, 255]).0))
        .expect("mounts");

    // When
    let depth = rig.depth_after(&on_transparent_canvas(), true);

    // Then: cleared to the far plane all the same
    assert!(depth.iter().all(|&d| d == 1.0), "{:?}", &depth[..8]);
}

#[test]
fn gpu_2d_draws_leave_the_depth_attachment_as_cleared() {
    // Given
    let mut rig = Rig::with_samples(Samples::One);
    rig.painter
        .mount("view", Box::new(Solid::new([0, 200, 0, 255]).0))
        .expect("mounts");

    // When: every 2D kind drawn into a pass with the depth attachment
    let depth = rig.depth_after(&on_transparent_canvas(), false);

    // Then: no 2D draw wrote depth
    let written = depth.iter().filter(|&&d| d != 1.0).count();
    assert_eq!(written, 0, "{written} texels written");
}

/// [`every_kind`] with no canvas background, which `draw_over` refuses.
fn on_transparent_canvas() -> vapors_elements::Document {
    let mut doc = every_kind();
    let css = "html, body { background-color: transparent; }";
    doc.add_stylesheet("transparent.css", css).expect("parses");
    support::update(&mut doc, 1.0);
    doc
}

/// The 2D pipeline families a painter holds, each in both depth
/// variants: quads, shadows and closed forms (instanced), meshes, the
/// image and text.
const FAMILIES: u64 = 6;

#[test]
fn gpu_a_painter_builds_each_2d_pipeline_in_both_depth_variants() {
    // Given
    let before = vapors_renderer::pipelines::built().pipelines;

    // When
    let _rig = Rig::with_samples(Samples::One);

    // Then: a pipeline added without its depth variant changes the count
    let built = vapors_renderer::pipelines::built().pipelines - before;
    assert_eq!(built, 2 * FAMILIES, "pipelines the painter built");
}
