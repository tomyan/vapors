//! An [`ElementsApp`] frame as design 02's phases (2-S1): the document's
//! update and its paint are the Update and Draw phases, and the time
//! its event handlers took since the last frame is the Input phase's.

use std::time::Duration;
use vapors_elements::Document;

use vapors_platform::{Frame, Instant};

use vapors_renderer::Target;

use vapors_elements::{ElementError, Viewport};
use vapors_scene::Size;

use super::ElementsApp;
use crate::frame::{FrameTimes, Phase, Phases, SystemClock, run_frame};
use crate::painter::ScenePainter;

/// The phases that do work today, shown by a per-frame hook.
const SHOWN: [Phase; 3] = [Phase::Input, Phase::Update, Phase::Draw];

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
        self.app.update_document(self.frame)
    }

    fn draw(&mut self, _now: Instant) -> Result<(), String> {
        self.app.paint(self.frame)
    }
}

impl ElementsApp {
    /// The Update phase: `Document::update`.
    fn update_document(&mut self, frame: &Frame<'_>) -> Result<(), String> {
        #[allow(clippy::cast_possible_truncation)] // a display scale
        let scale = frame.scale_factor as f32;
        let size = size_of(frame);
        let mut document = self.document.borrow_mut();
        document
            .update(viewport(size, scale))
            .map(|_| ())
            .map_err(|errors| refused(&errors))
    }

    /// The Draw phase: the update's scene painted into the frame.
    fn paint(&mut self, frame: &Frame<'_>) -> Result<(), String> {
        let size = size_of(frame);
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
            view: &view,
            viewport: size,
            scissor: None,
        };
        if self.painter.is_none() {
            let made = ScenePainter::new(frame.device, frame.texture.format())
                .map_err(|refused| refused.to_string())?;
            self.painter = Some(made);
        }
        let scene_painter = self.painter.as_mut().expect("made above");
        self.surfaces
            .bring_to(scene_painter)
            .map_err(|error| format!("the surfaces could not be mounted: {error}"))?;
        scene_painter
            .draw(&target, painted)
            .map_err(|error| format!("the document could not be painted: {error}"))
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

/// `times` for the shown phases, by name; empty before the first frame.
pub(super) fn shown_times(times: Option<&FrameTimes>) -> Vec<(&'static str, Duration)> {
    times
        .map(|times| {
            SHOWN
                .iter()
                .map(|phase| (phase.name(), times.of(*phase)))
                .collect()
        })
        .unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::shown_times;
    use crate::frame::{FrameTimes, Phase};

    #[test]
    fn the_shown_times_are_input_update_and_draw_once_there_is_a_frame() {
        // Given
        let times = FrameTimes::default();

        // Then
        assert_eq!(shown_times(None), Vec::new());
        let names: Vec<&str> = shown_times(Some(&times))
            .iter()
            .map(|(name, _)| *name)
            .collect();
        assert_eq!(
            names,
            [Phase::Input, Phase::Update, Phase::Draw].map(Phase::name)
        );
        assert!(
            shown_times(Some(&times))
                .iter()
                .all(|(_, time)| *time == Duration::ZERO)
        );
    }
}
