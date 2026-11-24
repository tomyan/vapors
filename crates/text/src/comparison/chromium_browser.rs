//! A browser kept for one test's screenshots: started on the first
//! screenshot at each device scale (`--force-device-scale-factor` is the
//! browser's), each screenshot in a browser context of its own
//! ([`super::chromium_context`]). A screenshot that fails or passes its
//! deadline stops its browser, so the next one starts a new browser and
//! no state of a broken one reaches it. Dropping the handle stops every
//! browser and removes its profile; [`Browser::close`] does the same and
//! reports what it could not do.

use std::path::Path;
use std::time::{Duration, Instant};

use super::chromium::{Chromium, ChromiumError};
use super::chromium_process::{check_size, remove_stale};
use super::chromium_running::Running;
use super::measure::Rgba;
use super::png_file::read_png;

/// Screenshots by browser context, in browsers that last as long as it.
pub struct Browser {
    chromium: Chromium,
    running: Vec<Running>,
    launches: usize,
}

impl Chromium {
    /// A handle for screenshots that share browsers. Nothing starts until
    /// its first screenshot.
    #[must_use]
    pub fn browser(&self) -> Browser {
        Browser {
            chromium: self.clone(),
            running: Vec::new(),
            launches: 0,
        }
    }
}

impl Browser {
    /// Screenshot `html` at `width` by `height` CSS pixels at `scale`
    /// device pixels per CSS pixel into `png`, and read it back. Starting
    /// the browser, when this screenshot needs one, counts within the
    /// screenshot's deadline, so one screenshot never takes longer than
    /// the one-shot launch could (`chromium_process_tests.rs` holds that
    /// under nextest's kill).
    ///
    /// # Errors
    /// [`ChromiumError::Failed`] when the browser cannot start, the
    /// screenshot fails or passes the deadline, a stale `png` cannot be
    /// removed first, or the PNG is not the requested size.
    pub fn screenshot(
        &mut self,
        html: &Path,
        png: &Path,
        width: u32,
        height: u32,
        scale: u32,
    ) -> Result<Rgba, ChromiumError> {
        self.shoot(html, png, [width, height], scale)
            .map_err(|error| self.chromium.named(error))
    }

    fn shoot(
        &mut self,
        html: &Path,
        png: &Path,
        size: [u32; 2],
        scale: u32,
    ) -> Result<Rgba, ChromiumError> {
        remove_stale(png)?;
        let until = Instant::now() + self.chromium.deadline;
        let at = self.started(size, scale, until)?;
        let deadline = self.chromium.deadline;
        let running = &mut self.running[at];
        let url = format!("file://{}", html.display());
        let shot = super::chromium_context::screenshot(
            &mut running.session,
            running.port,
            &url,
            size,
            until,
        );
        let bytes = shot.map_err(|why| self.stop(at, &within("the screenshot", &why, deadline)))?;
        std::fs::write(png, bytes)
            .map_err(|e| ChromiumError::Failed(format!("{}: {e}", png.display())))?;
        let image = read_png(png).map_err(ChromiumError::Failed)?;
        check_size(&image, png, [size[0] * scale, size[1] * scale])?;
        Ok(image)
    }

    /// The running browser at `scale`, started if there is none.
    fn started(
        &mut self,
        size: [u32; 2],
        scale: u32,
        until: Instant,
    ) -> Result<usize, ChromiumError> {
        if let Some(at) = self.running.iter().position(|r| r.scale == scale) {
            return Ok(at);
        }
        self.launches += 1;
        let running = Running::start(&self.chromium, size, scale, until).map_err(|e| match e {
            ChromiumError::Failed(why) => {
                ChromiumError::Failed(within("starting the browser", &why, self.chromium.deadline))
            }
            other @ ChromiumError::NotFound => other,
        })?;
        self.running.push(running);
        Ok(self.running.len() - 1)
    }

    /// Stop the browser at `at` after `why`.
    fn stop(&mut self, at: usize, why: &str) -> ChromiumError {
        let mut running = self.running.swap_remove(at);
        running.process.failed(why)
    }

    /// Browsers started so far.
    #[must_use]
    pub fn launches(&self) -> usize {
        self.launches
    }

    /// Stop every browser and remove its profile.
    ///
    /// # Errors
    /// A browser process outlived its kill, or a profile could not be
    /// removed, naming each.
    pub fn close(mut self) -> Result<(), ChromiumError> {
        let failures: Vec<String> = self
            .running
            .drain(..)
            .filter_map(|mut running| running.process.stop().err())
            .map(|e| e.to_string())
            .collect();
        if failures.is_empty() {
            return Ok(());
        }
        Err(ChromiumError::Failed(failures.join("\n")))
    }
}

/// `why` `what` failed, with the deadline it had.
fn within(what: &str, why: &str, deadline: Duration) -> String {
    format!("{what}, with {}s to finish: {why}", deadline.as_secs())
}

#[cfg(test)]
#[path = "chromium_browser_tests.rs"]
pub(crate) mod tests;
