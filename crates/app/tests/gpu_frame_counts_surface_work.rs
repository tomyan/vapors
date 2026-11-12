//! GPU tier: what a `gpu-surface`'s renderer issues through
//! `SurfaceFrame::work` is added to `FrameCounts::surfaces`
//! (docs/plans/3d-2.md decision 8), on the frames it renders.

#[path = "support/surface.rs"]
mod support;

use support::{Rig, bare_document, update};
use vapors_app::{FrameCounts, SurfaceError, SurfaceFrame, SurfaceRenderer};
use vapors_elements::Document;

/// An image quad: 6 vertices of 4 floats.
const IMAGE_QUAD_BYTES: u64 = 6 * 16;
/// What [`Writer`] writes each render.
const WRITTEN: u64 = 64;

/// Writes [`WRITTEN`] bytes to a buffer of its own each render, counted.
#[derive(Default)]
struct Writer {
    buffer: Option<wgpu::Buffer>,
}

impl SurfaceRenderer for Writer {
    fn mount(&mut self, device: &wgpu::Device) -> Result<(), SurfaceError> {
        self.buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("writer"),
            size: WRITTEN,
            usage: wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        Ok(())
    }

    fn render(&mut self, frame: SurfaceFrame<'_>) -> Result<(), SurfaceError> {
        let buffer = self.buffer.as_ref().expect("mounted");
        let bytes = [0; 64];
        frame.work.write_buffer(frame.queue, buffer, 0, &bytes);
        Ok(())
    }
}

fn surface_document() -> Document {
    let css = ".track { width: 16px; height: 16px; }";
    let mut doc = bare_document(css, &[(None, "gpu-surface", "track")]);
    update(&mut doc, 1.0);
    doc
}

fn drawn(rig: &mut Rig, doc: &Document) -> FrameCounts {
    rig.draw(doc).expect("draws");
    rig.painter.frame_counts()
}

#[test]
fn gpu_a_surface_render_adds_its_counted_work_to_the_surface_counts() {
    // Given
    let doc = surface_document();
    let mut rig = Rig::new();
    rig.painter
        .mount("track", Box::<Writer>::default())
        .expect("mounts");

    // When
    let counts = drawn(&mut rig, &doc);

    // Then
    assert_eq!(counts.surfaces.buffer_bytes, IMAGE_QUAD_BYTES + WRITTEN);
    assert_eq!(counts.surfaces.draw_calls, 1);
}

#[test]
fn gpu_a_frame_that_does_not_render_the_surface_counts_only_its_composition() {
    // Given
    let doc = surface_document();
    let mut rig = Rig::new();
    rig.painter
        .mount("track", Box::<Writer>::default())
        .expect("mounts");
    drawn(&mut rig, &doc);

    // When
    let counts = drawn(&mut rig, &doc);

    // Then: the placing draw alone, and no frame after it
    assert_eq!(counts.surfaces.draw_calls, 1);
    assert_eq!(counts.surfaces.buffer_bytes, IMAGE_QUAD_BYTES);
    assert!(!rig.painter.wants_frame());
}
