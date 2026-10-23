//! The platform `App` for a document: every frame updates it at the
//! window's size and scale and paints the result. A refused update or a
//! failed paint is reported (stderr natively, `#error` on the web), kept
//! for the caller in a [`Failure`], and stops the loop: the window never
//! shows a frame the document refused.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_elements::{Document, ElementError, Viewport};
use vapors_platform::{App, Event, Frame, Next, Platform, report_error};
use vapors_renderer::Target;
use vapors_scene::Size;

use crate::painter::ScenePainter;

/// Why the app stopped, once it has: shared with whoever ran it.
#[derive(Clone, Default)]
pub struct Failure(Rc<RefCell<Option<String>>>);

impl Failure {
    /// The report, if the app stopped on a failure.
    #[must_use]
    pub fn get(&self) -> Option<String> {
        self.0.borrow().clone()
    }

    fn set(&self, message: &str) {
        *self.0.borrow_mut() = Some(message.to_owned());
    }
}

/// Shows a document in the window.
pub struct ElementsApp {
    document: Rc<RefCell<Document>>,
    painter: Option<ScenePainter>,
    failure: Failure,
}

impl ElementsApp {
    /// Shows `document`.
    #[must_use]
    pub fn new(document: Rc<RefCell<Document>>) -> Self {
        Self {
            document,
            painter: None,
            failure: Failure::default(),
        }
    }

    /// Where the app's failure is kept; clone it before `run` takes the app.
    #[must_use]
    pub fn failure(&self) -> Failure {
        self.failure.clone()
    }

    /// Updates the document for this frame and paints it into the frame.
    ///
    /// # Errors
    ///
    /// The update's refusals, or the painter's error, as one report.
    pub fn draw(&mut self, frame: &Frame<'_>) -> Result<(), String> {
        let size = Size {
            width: frame.texture.width(),
            height: frame.texture.height(),
        };
        let mut document = self.document.borrow_mut();
        let painted = document
            .update(viewport(size, frame.scale_factor))
            .map_err(|errors| refused(&errors))?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let target = Target {
            device: frame.device,
            queue: frame.queue,
            view: &view,
            viewport: size,
        };
        let scene_painter = self
            .painter
            .get_or_insert_with(|| ScenePainter::new(frame.device, frame.texture.format()));
        scene_painter
            .draw(&target, painted)
            .map_err(|error| format!("the document could not be painted: {error}"))
    }
}

impl App for ElementsApp {
    /// Pointer input reaches the document in F2c S2; until then events
    /// change nothing it shows.
    fn event(&mut self, _event: Event, _platform: &Platform) {}

    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        match self.draw(frame) {
            Ok(()) => Next::Continue,
            Err(message) => {
                report_error(&message);
                self.failure.set(&message);
                Next::Exit
            }
        }
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)] // window sizes and scales are small
fn viewport(size: Size, scale: f64) -> Viewport {
    let scale = scale as f32;
    Viewport {
        width: size.width as f32 / scale,
        height: size.height as f32 / scale,
        scale,
    }
}

fn refused(errors: &[ElementError]) -> String {
    let lines: Vec<String> = errors.iter().map(|error| format!("  {error}")).collect();
    format!(
        "the document was refused, so nothing is drawn:\n{}",
        lines.join("\n")
    )
}
