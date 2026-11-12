//! An [`ElementsApp`] frame as design 02's phases (2-S1): the document's
//! update and its paint are the Update and Draw phases, the a11y
//! mirror's take (`elements_app_a11y.rs`) the Accessibility phase, and
//! the time its event handlers took since the last frame is the Input
//! phase's.

use std::time::Duration;

use vapors_elements::{Document, ElementError, Viewport};
use vapors_platform::{Frame, Instant};
use vapors_renderer::{FrameEncoder, Target};
use vapors_scene::Size;

use super::ElementsApp;
use super::shown::shown_times;
use crate::check_tooltip_style;
use crate::frame::{Phases, SystemClock, run_frame};

/// One frame of an app.
struct AppFrame<'a, 'f> {
    app: &'a mut ElementsApp,
    frame: &'a Frame<'f>,
}

impl Phases for AppFrame<'_, '_> {
    fn input_time(&mut self) -> Duration {
        std::mem::take(&mut self.app.input_spent)
    }

    fn update(&mut self, _now: Instant) -> Result<(), String> {
        // the selectionchange each text field owes, once per frame, as
        // Chromium fires one per task (E6a)
        Document::settle_selection(&self.app.document);
        self.app.update_document(self.frame)?;
        // the focus fixup: an element the update left without a box, or
        // disabled, loses focus before the next event can reach it
        Document::settle_focus(&self.app.document);
        Ok(())
    }

    fn accessibility(&mut self, _now: Instant) -> Result<(), String> {
        self.app.take_a11y()
    }

    fn draw(&mut self, now: Instant) -> Result<(), String> {
        self.app.paint(self.frame, now)
    }
}

impl ElementsApp {
    /// The Update phase: `Document::update`, then the tooltip's style
    /// checked, so a bad `--tooltip-*` value fails this frame and not
    /// the first hover (seven lookups on the root's custom properties).
    fn update_document(&mut self, frame: &Frame<'_>) -> Result<(), String> {
        #[allow(clippy::cast_possible_truncation)] // a display scale
        let scale = frame.scale_factor as f32;
        let size = size_of(frame);
        let mut document = self.document.borrow_mut();
        document.set_max_image_side(frame.device.limits().max_texture_dimension_2d);
        document
            .update(viewport(size, scale))
            .map_err(|errors| refused(&errors))?;
        check_tooltip_style(&document)
            .map_err(|error| format!("the document was refused, so nothing is drawn: {error}"))
    }

    /// The Draw phase: the update's scene painted into the frame, then
    /// the tooltip shown at `now` over it, in one submit. A refused
    /// frame submits what it recorded before the refusal.
    fn paint(&mut self, frame: &Frame<'_>, now: Instant) -> Result<(), String> {
        let encoder = self.timer.encoder(frame);
        let painted = self.record(frame, &encoder, now);
        self.render_passes = encoder.render_passes();
        let submitted = encoder.submit(frame.queue);
        painted.and(submitted.map_err(|refused| refused.to_string()))
    }

    fn record(
        &mut self,
        frame: &Frame<'_>,
        encoder: &FrameEncoder,
        now: Instant,
    ) -> Result<(), String> {
        let document = self.document.borrow();
        let painted = document
            .painted()
            .ok_or_else(|| "nothing to draw: the document has not been updated".to_owned())?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let target = Target {
            device: frame.device,
            queue: frame.queue,
            encoder,
            view: &view,
            viewport: size_of(frame),
            scissor: None,
        };
        if self.painter.is_none() {
            let made = self
                .make_painter(frame)
                .map_err(|refused| refused.to_string())?;
            self.painter = Some(made);
        }
        let scene_painter = self.painter.as_mut().expect("made above");
        self.surfaces
            .bring_to(scene_painter)
            .map_err(|error| format!("the surfaces could not be mounted: {error}"))?;
        scene_painter
            .draw(&target, painted)
            .map_err(|error| format!("the document could not be painted: {error}"))?;
        drop(document);
        self.paint_tooltip(&target, now)
    }

    /// The tooltip shown at `now`, if any, over the painted scene.
    /// Asking also shows one whose deadline passed, which ends its wait.
    fn paint_tooltip(&mut self, target: &Target<'_>, now: Instant) -> Result<(), String> {
        let mut document = self.document.borrow_mut();
        let Some(tooltip) = self.input.tooltip(&document, now).cloned() else {
            return Ok(());
        };
        let painter = self.painter.as_mut().expect("made by paint");
        painter
            .draw_tooltip(
                target,
                &mut document,
                &tooltip,
                self.input.tooltip_hotspot(),
            )
            .map_err(|error| error.to_string())
    }

    /// Runs the frame's phases, keeping their times and handing them to
    /// the frame-times hook.
    pub(super) fn run_phases(&mut self, frame: &Frame<'_>) -> Result<(), String> {
        // the app's own per-frame writes run before the frame, so its
        // phase times hold the frame's work alone (review-v75, M2)
        if let Some(per_frame) = &mut self.per_frame {
            per_frame(&shown_times(self.last_times.as_ref()));
        }
        let mut phases = AppFrame { app: self, frame };
        let times =
            run_frame(&mut phases, frame.now, &SystemClock).map_err(|error| error.to_string())?;
        self.last_times = Some(times);
        if let Some(hook) = &mut self.frame_times {
            hook(&times);
        }
        Ok(())
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

fn size_of(frame: &Frame<'_>) -> Size {
    Size {
        width: frame.texture.width(),
        height: frame.texture.height(),
    }
}

fn refused(errors: &[ElementError]) -> String {
    let lines: Vec<String> = errors.iter().map(|error| format!("  {error}")).collect();
    format!(
        "the document was refused, so nothing is drawn:\n{}",
        lines.join("\n")
    )
}
