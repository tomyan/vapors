//! GPU tier: a kept batch drawn with a range holding fewer rows than
//! its kept bytes name is refused on the draw from kept bytes, in every
//! build (`rows_fit`), on both paths: the instanced and image pipelines'
//! (`painter_kept_gpu.rs` `range`) and text's (`painter_kept_text.rs`
//! `text_range`). The rows are given directly: vapors-elements makes no
//! page whose range shrinks under kept bytes.

use vapors_dom::Dom;
use vapors_elements::{Document, KeptBatched, Painted, Viewport};
use vapors_gpu_harness::gpu;
use vapors_renderer::{FrameEncoder, Target};
use vapors_scene::{Rect, Run, Size};
use vapors_text::FontSystem;

use super::Pipelines;
use crate::ScenePainter;

const SIDE: u32 = 16;

/// Runs `test` on a page of one red div labelled "Hi": a quad batch
/// and a text batch.
fn on_page(test: impl FnOnce(&Painted, &KeptBatched)) {
    let mut doc = Document::new(FontSystem::bundle_only());
    let css = "div { height: 8px; font-size: 7px; background-color: #f00; color: #000; }";
    doc.add_stylesheet("app.css", css).expect("parses");
    let div = doc.create_element("div");
    let label = doc.create_text("Hi");
    doc.append(div, label);
    doc.append(doc.body(), div);
    let side = f32::from(u16::try_from(SIDE).expect("small"));
    let view = Viewport {
        width: side,
        height: side,
        scale: 1.0,
    };
    let painted = doc.update(view).expect("updates");
    let kept = painted.kept_batches(Rect::new(0.0, 0.0, side, side));
    test(painted, &kept);
}

/// The batch of `kept` whose run `is` accepts.
fn batch_where(kept: &KeptBatched, is: fn(&Run) -> bool) -> usize {
    let found = kept.batched.batches.iter().position(|batch| is(&batch.run));
    found.expect("the page has the batch")
}

/// Runs `draw` with a painter and a target on a fresh square.
fn on_target(draw: impl FnOnce(&mut ScenePainter, &Target<'_>)) {
    let gpu = gpu();
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("rows fit"),
        size: wgpu::Extent3d {
            width: SIDE,
            height: SIDE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let encoder = FrameEncoder::new(&gpu.device);
    let target = Target {
        device: &gpu.device,
        queue: &gpu.queue,
        encoder: &encoder,
        view: &view,
        multisampled: None,
        depth: None,
        viewport: Size {
            width: SIDE,
            height: SIDE,
        },
        scissor: None,
    };
    let mut painter = ScenePainter::new(&gpu.device, format).expect("a supported format");
    draw(&mut painter, &target);
}

#[test]
#[should_panic(expected = "kept bytes name 3 rows, and its range holds 2")]
fn gpu_kept_quads_drawn_with_a_range_short_of_their_rows_are_refused() {
    on_page(|_, kept| {
        let at = batch_where(kept, |run| matches!(run, Run::Quads(_)));
        let (run, key) = (&kept.batched.batches[at].run, kept.keys[at]);
        on_target(|painter, target| {
            for range in [3, 2] {
                let pipelines = pipelines!(painter);
                let drawn = (run, ((0, &[][..]), &[2][..], range), key, at);
                let _ = painter
                    .kept
                    .range(pipelines, target, drawn)
                    .expect("builds");
            }
        });
    });
}

#[test]
#[should_panic(expected = "kept bytes name 3 rows, and its range holds 2")]
fn gpu_kept_text_drawn_with_a_range_short_of_its_rows_is_refused() {
    on_page(|painted, kept| {
        let at = batch_where(kept, |run| matches!(run, Run::Texts(_)));
        let (run, key) = (&kept.batched.batches[at].run, kept.keys[at]);
        on_target(|painter, target| {
            for range in [3, 2] {
                let parts = (&mut painter.text, &mut painter.atlas);
                let drawn = (run, ((0, &[][..]), &[2][..], range), key, at);
                let _ = painter
                    .kept
                    .text_range(parts, (target, painted), drawn)
                    .expect("places its glyphs");
            }
        });
    });
}
