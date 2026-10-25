//! The painter's images: each uploaded once, while frames keep drawing
//! it, and drawn stretched over its item's rect.

use std::rc::Rc;

use vapors_elements::Painted;
use vapors_renderer::{ImagePipeline, Target, upload_rgba8};
use vapors_scene::{Item, RgbaImage};

/// The images uploaded so far, by the image they hold (compared by
/// identity: one registration is one upload).
#[derive(Default)]
pub(super) struct Textures {
    uploaded: Vec<(Rc<RgbaImage>, wgpu::Texture, bool)>,
}

impl Textures {
    /// Marks every texture unused, before a frame.
    pub(super) fn begin_frame(&mut self) {
        for (_, _, used) in &mut self.uploaded {
            *used = false;
        }
    }

    /// Drops the textures the last frame did not draw.
    pub(super) fn end_frame(&mut self) {
        self.uploaded.retain(|(_, _, used)| *used);
    }

    /// Draws each image item of `items` with `pipeline`.
    pub(super) fn draw(
        &mut self,
        pipeline: &ImagePipeline,
        target: &Target<'_>,
        painted: &Painted,
        items: &[Item],
    ) {
        for item in items {
            if let Item::Image { image, rect } = item {
                let texture = self.texture(target, painted.image(*image));
                pipeline.draw_rect(target, texture, *rect);
            }
        }
    }

    fn texture(&mut self, target: &Target<'_>, image: &Rc<RgbaImage>) -> &wgpu::Texture {
        let found = self
            .uploaded
            .iter()
            .position(|(held, _, _)| Rc::ptr_eq(held, image));
        let at = found.unwrap_or_else(|| {
            let size = image.size;
            let texture = upload_rgba8(
                target.device,
                target.queue,
                size.width,
                size.height,
                &image.rgba,
            );
            self.uploaded.push((Rc::clone(image), texture, false));
            self.uploaded.len() - 1
        });
        self.uploaded[at].2 = true;
        &self.uploaded[at].1
    }
}
