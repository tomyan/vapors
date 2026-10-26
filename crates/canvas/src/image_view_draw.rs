//! Drawing the view: clear to the background, then hand the frame to the
//! app's painter, or upload the image once and draw it under the current
//! transform.

use vapors_platform::Frame;
use vapors_renderer::{ImagePipeline, clear_view, upload_rgba8};

use crate::ImageView;
use crate::image_view::Uploaded;
use crate::image_view_painter::Shown;

impl ImageView {
    /// Clear the frame to the background and draw the image, if any.
    ///
    /// # Errors
    ///
    /// The painter's error, prefixed `image painter: `, when an
    /// [`crate::ImagePainter`] could not draw; the frame then shows the
    /// background, or what the painter drew before it failed.
    pub fn draw(&mut self, frame: &Frame<'_>) -> Result<(), String> {
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        clear_view(frame.device, frame.queue, &view, self.background);
        if let Some(painted) = self.paint(frame, &view) {
            return painted.map_err(|why| format!("image painter: {why}"));
        }
        let (viewport, transform) = (self.viewport, self.transform());
        let Some(uploaded) = self.upload(frame) else {
            return Ok(());
        };
        uploaded.pipeline.draw(
            frame.device,
            frame.queue,
            &view,
            viewport,
            &uploaded.texture,
            transform,
        );
        Ok(())
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
