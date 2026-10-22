//! The image app: one [`ImageView`], the keys and gestures that steer it,
//! and the file picker to put an image in it. The native example and the
//! web demo both run this.

use vapors_platform::{App, Event, Frame, Next, Platform, report_error};
use vapors_scene::Color;

use crate::{ImageView, RgbaImage, decode_png};

/// Shows one image. `o` opens the picker; so does a tap or click while
/// nothing is shown, for a phone without a keyboard.
pub struct ImageApp {
    view: ImageView,
}

impl ImageApp {
    /// Nothing shown yet: the background, waiting for a file.
    #[must_use]
    pub fn new(background: Color) -> Self {
        Self {
            view: ImageView::empty(background),
        }
    }

    /// Showing `image`, fitted.
    #[must_use]
    pub fn showing(image: RgbaImage, background: Color) -> Self {
        Self {
            view: ImageView::new(image, background),
        }
    }

    /// The view, for its transform.
    #[must_use]
    pub fn view(&self) -> &ImageView {
        &self.view
    }

    fn opens_picker(&self, event: &Event) -> bool {
        match event {
            Event::Character(text) => text == "o",
            Event::Tap { .. } | Event::PrimaryButton { pressed: false } => !self.view.has_image(),
            _ => false,
        }
    }

    fn file_opened(&mut self, name: &str, bytes: &[u8]) {
        match decode_png(bytes) {
            Ok(image) => self.view.set_image(image),
            Err(message) => report_error(&format!("{name}: {message}")),
        }
    }
}

impl App for ImageApp {
    fn event(&mut self, event: Event, platform: &Platform) {
        if self.opens_picker(&event) {
            platform.pick_file();
        }
        if let Event::FileOpened { name, bytes } = &event {
            self.file_opened(name, bytes);
        }
        self.view.event(&event);
    }

    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        self.view.draw(frame);
        Next::Continue
    }
}

#[cfg(test)]
#[path = "image_app_tests.rs"]
mod tests;
