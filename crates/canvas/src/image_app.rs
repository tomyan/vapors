//! The image app: one [`ImageView`], the keys and gestures that steer it,
//! the file picker to put an image in it, and the label strip naming the
//! file and the zoom. The native example and the web demo both run this.

use vapors_platform::{App, Event, FileKind, Frame, Next, Platform, report_error, report_status};
use vapors_scene::Color;
use vapors_text::FontSystem;

use crate::label::{LabelDraw, label_text};
use crate::{ImageView, RgbaImage, decode_png};

/// Shows one image. `o` opens the picker; so does a tap or click while
/// nothing is shown, for a phone without a keyboard.
pub struct ImageApp {
    view: ImageView,
    file_name: Option<String>,
    fonts: FontSystem,
    label: Option<LabelDraw>,
}

impl ImageApp {
    /// Nothing shown yet: the background, waiting for a file.
    #[must_use]
    pub fn new(background: Color) -> Self {
        Self {
            view: ImageView::empty(background),
            file_name: None,
            fonts: FontSystem::new(),
            label: None,
        }
    }

    /// Showing `image` from the file called `name`, fitted.
    #[must_use]
    pub fn showing(name: &str, image: RgbaImage, background: Color) -> Self {
        let mut app = Self::new(background);
        app.view.set_image(image);
        app.file_name = Some(name.to_owned());
        app
    }

    /// The view, for its transform.
    #[must_use]
    pub fn view(&self) -> &ImageView {
        &self.view
    }

    /// What the label strip says right now.
    #[must_use]
    pub fn label(&self) -> String {
        label_text(self.file_name.as_deref(), self.view.transform().scale)
    }

    /// What the picker offers. Decoding knows PNG only, so far.
    fn file_kinds() -> Vec<FileKind> {
        vec![FileKind::new("PNG image", ["png"])]
    }

    fn opens_picker(&self, event: &Event) -> bool {
        match event {
            Event::Character(text) => text == "o",
            Event::OpenRequested => true,
            Event::Tap { .. } | Event::PrimaryButton { pressed: false } => !self.view.has_image(),
            _ => false,
        }
    }

    fn file_opened(&mut self, name: &str, bytes: &[u8]) {
        match decode_png(bytes) {
            Ok(image) => {
                report_status(&format!(
                    "opened {name} {}x{}",
                    image.size.width, image.size.height
                ));
                self.view.set_image(image);
                self.file_name = Some(name.to_owned());
            }
            Err(message) => report_error(&format!("{name}: {message}")),
        }
    }
}

impl App for ImageApp {
    fn event(&mut self, event: Event, platform: &Platform) {
        if self.opens_picker(&event)
            && let Err(error) = platform.pick_file(&Self::file_kinds())
        {
            report_error(&error.to_string());
        }
        if let Event::FileOpened { name, bytes } = &event {
            self.file_opened(name, bytes);
        }
        self.view.event(&event);
    }

    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        self.view.draw(frame);
        let text = self.label();
        let label = self
            .label
            .get_or_insert_with(|| LabelDraw::new(frame.device, frame.texture.format()));
        label.draw(frame, &mut self.fonts, &text);
        Next::Continue
    }
}

#[cfg(test)]
#[path = "image_app_tests.rs"]
mod tests;
