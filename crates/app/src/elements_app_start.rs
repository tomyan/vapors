//! The app before its first frame (`App::start`, one-scene F0): its
//! painter made, every pipeline with it, its surface renderers mounted
//! and the glyph atlas's new page uploaded, so no shown frame pays for
//! them. An app whose frames come without a start (tests that drive
//! `App::frame` alone) makes the painter in its first frame instead;
//! the judged run (`vapors-perf --scene-run`) refuses a frame that
//! builds a pipeline, so a missing start is found there.

use vapors_platform::Start;

use super::ElementsApp;

impl ElementsApp {
    /// [`App::start`](vapors_platform::App::start) for this app.
    pub(super) fn start_on(&mut self, start: &Start<'_>) -> Result<(), String> {
        let painter = self.make_painter(start.device, start.format);
        let painter = self
            .painter
            .insert(painter.map_err(|refused| refused.to_string())?);
        self.surfaces
            .bring_to(painter)
            .map_err(|error| format!("the surfaces could not be mounted: {error}"))?;
        painter
            .start(start.device, start.queue)
            .map_err(|error| format!("a surface could not be mounted: {error}"))
    }

    /// Refused when the painter, made at start or by an earlier frame,
    /// draws into another format than `format`.
    pub(super) fn check_format(&self, format: wgpu::TextureFormat) -> Result<(), String> {
        match &self.painter {
            Some(painter) if painter.format() != format => Err(format!(
                "the frame's texture is {format:?} but the app started for {:?}; fix: \
                 give App::start the format every frame's texture has",
                painter.format()
            )),
            _ => Ok(()),
        }
    }
}
