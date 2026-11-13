//! The painter's images: each uploaded and bound once, while frames
//! keep drawing it, and drawn stretched over its item's rect, from the
//! run's quads kept in the image pipeline (`painter_kept.rs`). The
//! binding is kept with the texture and dropped with it: an image
//! registered again is another texture with its own binding.

use std::rc::Rc;

use vapors_elements::Painted;
use vapors_renderer::{
    GpuWork, ImagePipeline, KeptRange, Sampling, Shift, Target, TextureBinding, WorkCounter,
    upload_rgba8_mipmapped_counted,
};
use vapors_scene::{Item, RgbaImage};

/// The images uploaded so far, by the image they hold (compared by
/// identity: one registration is one upload).
#[derive(Default)]
pub(super) struct Textures {
    uploaded: Vec<(Rc<RgbaImage>, TextureBinding, bool)>,
    /// The uploads' bytes.
    work: WorkCounter,
}

impl Textures {
    /// Marks every texture unused, before a frame.
    pub(super) fn begin_frame(&mut self) {
        for (_, _, used) in &mut self.uploaded {
            *used = false;
        }
    }

    /// The bytes uploaded since the last take; the count restarts.
    pub(super) fn take_work(&self) -> GpuWork {
        self.work.take()
    }

    /// Drops the textures the last frame did not draw, their bindings
    /// with them.
    pub(super) fn end_frame(&mut self) {
        self.uploaded.retain(|(_, _, used)| *used);
    }

    /// The bindings kept: one per texture.
    pub(super) fn bind_groups_kept(&self) -> u64 {
        self.uploaded.len() as u64
    }

    /// Draws each image item of `items` with `pipeline`, over the quad
    /// at its place in `kept`, moved by `shift`.
    pub(super) fn draw_kept(
        &mut self,
        pipeline: &mut ImagePipeline,
        target: &Target<'_>,
        painted: &Painted,
        (items, kept, shift): (&[Item], &KeptRange, Shift),
    ) {
        for (place, item) in (0..).zip(items) {
            if let Item::Image { image, .. } = item {
                let bound = self.bound(pipeline, target, painted.image(*image));
                pipeline.draw_kept(target, bound, (kept, place), shift);
            }
        }
    }

    /// `image`'s texture and binding, uploaded and bound on first use.
    fn bound(
        &mut self,
        pipeline: &ImagePipeline,
        target: &Target<'_>,
        image: &Rc<RgbaImage>,
    ) -> &TextureBinding {
        let found = self
            .uploaded
            .iter()
            .position(|(held, _, _)| Rc::ptr_eq(held, image));
        let at = found.unwrap_or_else(|| {
            let texture = upload(target, image, &self.work);
            let bound = pipeline.bind(target.device, &texture, Sampling::Smooth);
            self.uploaded.push((Rc::clone(image), bound, false));
            self.uploaded.len() - 1
        });
        self.uploaded[at].2 = true;
        &self.uploaded[at].1
    }
}

fn upload(target: &Target<'_>, image: &RgbaImage, work: &WorkCounter) -> wgpu::Texture {
    let size = [image.size.width, image.size.height];
    upload_rgba8_mipmapped_counted(target.device, target.queue, size, &image.rgba, work)
}
