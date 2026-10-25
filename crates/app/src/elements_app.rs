//! The platform `App` for a document: every frame updates it at the
//! window's size and scale and paints the result. A refused update or a
//! failed paint is reported (stderr natively, `#error` on the web), kept
//! for the caller in a [`Failure`], and stops the loop: the window never
//! shows a frame the document refused.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use vapors_elements::{Cursor, Document, ElementError, Point, Viewport};
use vapors_platform::{App, Event, Frame, Instant, Next, Platform, report_error};
use vapors_renderer::Target;
use vapors_scene::Size;

use crate::painter::ScenePainter;
use crate::pointer::{Action, action};

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
    /// The last frame's display scale, for input in physical pixels.
    scale: f32,
    /// Where the pointer last moved, in CSS pixels.
    pointer: Point,
    touch: crate::touch_scroll::TouchScroll,
    /// The cursor last shown.
    cursor: Cursor,
    /// Called before each update with the last one's duration.
    per_frame: Option<PerFrame>,
    /// How long the last `Document::update` took; `None` before the first.
    last_update: Option<Duration>,
}

/// What [`ElementsApp::with_per_frame`] calls before each update, with
/// how long the last update took (`None` before the first).
pub type PerFrame = Box<dyn FnMut(Option<Duration>)>;

impl ElementsApp {
    /// Shows `document`.
    #[must_use]
    pub fn new(document: Rc<RefCell<Document>>) -> Self {
        Self {
            document,
            painter: None,
            failure: Failure::default(),
            scale: 1.0,
            pointer: [0.0, 0.0],
            touch: crate::touch_scroll::TouchScroll::default(),
            cursor: Cursor::Default,
            per_frame: None,
            last_update: None,
        }
    }

    /// Calls `per_frame`, when there is one, before each update with how
    /// long the last update took: for a demo that changes the document
    /// every frame and shows its own update time (`stress`). What it
    /// changes asks for the next frame, as any change does.
    #[must_use]
    pub fn with_per_frame(mut self, per_frame: Option<PerFrame>) -> Self {
        self.per_frame = per_frame;
        self
    }

    /// Takes input as if the display scale were `scale`, until the first
    /// frame says what it is: for driving `event` without a window.
    #[must_use]
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
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
        #[allow(clippy::cast_possible_truncation)] // a display scale
        let scale = frame.scale_factor as f32;
        self.scale = scale;
        if let Some(per_frame) = &mut self.per_frame {
            per_frame(self.last_update);
        }
        let mut document = self.document.borrow_mut();
        let started = Instant::now();
        let updated = document.update(viewport(size, scale));
        self.last_update = Some(started.elapsed());
        let painted = updated.map_err(|errors| refused(&errors))?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let target = Target {
            device: frame.device,
            queue: frame.queue,
            view: &view,
            viewport: size,
            scissor: None,
        };
        let scene_painter = self
            .painter
            .get_or_insert_with(|| ScenePainter::new(frame.device, frame.texture.format()));
        scene_painter
            .draw(&target, painted)
            .map_err(|error| format!("the document could not be painted: {error}"))
    }
}

impl ElementsApp {
    /// What to do after a frame: draw again at once if the document
    /// changed since it was last asked (by something outside an event,
    /// or by the update itself), else wait for the next change.
    pub fn next_after_frame(&mut self) -> Next {
        if self.document.borrow_mut().take_changed() {
            Next::Continue
        } else {
            Next::Wait
        }
    }

    fn act(&mut self, action: Action) {
        let document = &self.document;
        match action {
            Action::Move(point) => {
                self.pointer = point;
                Document::pointer_moved(document, point);
            }
            Action::Press(point) => Document::pointer_pressed(document, point),
            Action::Release(point) => Document::pointer_released(document, point),
            Action::Leave => Document::pointer_left(document),
            Action::Tap(point) => Document::tapped(document, point),
            Action::Wheel(point, delta) => _ = document.borrow_mut().scroll_by(point, delta),
            Action::TouchDown(point) => self.touch.down(point),
            Action::Pan(delta) => _ = self.touch.pan(&mut document.borrow_mut(), delta),
            Action::TouchUp => self.touch.up(),
        }
    }
}

/// Draws only when the document changed: an idle document draws no
/// frames.
impl App for ElementsApp {
    fn event(&mut self, event: Event, platform: &Platform) {
        if let Some(action) = action(&event, self.scale, self.pointer) {
            self.act(action);
        }
        if self.document.borrow_mut().take_changed() {
            platform.request_redraw();
        }
        let cursor = self.document.borrow().cursor(self.pointer);
        if cursor != self.cursor {
            self.cursor = cursor;
            platform.set_cursor(crate::cursor::shown(cursor));
        }
    }

    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        match self.draw(frame) {
            Ok(()) => self.next_after_frame(),
            Err(message) => {
                report_error(&message);
                self.failure.set(&message);
                Next::Exit
            }
        }
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)] // window sizes and scales are small
fn viewport(size: Size, scale: f32) -> Viewport {
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
