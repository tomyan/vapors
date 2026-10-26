//! Drawing the view: clear to the background, then hand the frame to the
//! app's painter, or upload the image once and draw it under the current
//! transform.

use vapors_platform::Frame;
use vapors_renderer::{ImagePipeline, clear_view, upload_rgba8};
use vapors_scene::Color;

use crate::ImageView;
use crate::image_view::Uploaded;
use crate::image_view_painter::Shown;

impl ImageView {
    /// Clear the frame to the background and draw the image, if any.
    pub fn draw(&mut self, frame: &Frame<'_>) {
        clear_frame(frame, self.background);
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        if self.paint(frame, &view) {
            return;
        }
        let (viewport, transform) = (self.viewport, self.transform());
        let Some(uploaded) = self.upload(frame) else {
            return;
        };
        uploaded.pipeline.draw(
            frame.device,
            frame.queue,
            &view,
            viewport,
            &uploaded.texture,
            transform,
        );
    }

    fn upload(&mut self, frame: &Frame<'_>) -> Option<&Uploaded> {
        let Some(Shown::Pixels(image)) = self.shown.as_ref() else {
            return None;
        };
        Some(self.uploaded.get_or_insert_with(|| Uploaded {
            pipeline: ImagePipeline::new(frame.device, frame.texture.format()),
            texture: upload_rgba8(
                frame.device,
                frame.queue,
                image.size.width,
                image.size.height,
                &image.rgba,
            ),
        }))
    }
}

fn clear_frame(frame: &Frame<'_>, color: Color) {
    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    clear_view(frame.device, frame.queue, &view, color);
}
