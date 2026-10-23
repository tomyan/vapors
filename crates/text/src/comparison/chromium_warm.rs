//! The browser's first launch on a fresh machine, taken before the
//! screenshots whose deadline assumes a warm one.

use std::path::Path;
use std::time::{Duration, Instant};

use super::chromium::{Chromium, ChromiumError};

/// The first launch on a fresh runner. Through `Chromium::find()`, which
/// picks the runner's `chromium` snapshot (Chromium 155.0.8059.0, not
/// Google Chrome 155.0.8059.39), a blank warm-up took 36.2 s in run
/// 40023109923 (#34), and the 1x paint test, the job's first launch
/// before the warm-up, took 15.4 s in run 40020237373. Google Chrome's
/// first launch took 1.26 s blank and
/// 12.7 to 20.2 s with text (runs 40018468987, 40019150781, 40020711297).
/// Every later launch took 0.6 to 0.9 s. Five runs of this warm-up on
/// vapors #40 (run 40026130114) took 22.3, 33.5, 57.9, 15.3 and 35.9 s.
/// 145 s is 2.5 times the worst, 57.9 s (job 120665457660). The cost is the browser's own first run, spread over start-up;
/// building fontconfig's cache first does not remove it (0.22 s, then
/// 20.2 s, run 40020711297).
const COLD_START: Duration = Duration::from_secs(145);

/// Text in a system font, so the first launch's text start-up is paid here.
const WARM_PAGE: &str =
    "<!doctype html><p style=\"font-family: sans-serif\">Warming the browser.</p>";

impl Chromium {
    /// One screenshot of a page of text in `work`, with the cold-start
    /// deadline, so later screenshots meet their warm one. Returns how
    /// long it took. The page renders text, so the first text start-up is
    /// paid here too (Google Chrome: 1.26 s blank, 12.7 to 20.2 s with
    /// text).
    ///
    /// # Errors
    ///
    /// [`ChromiumError::Failed`] when the page cannot be written, or the
    /// screenshot fails or passes the cold-start deadline.
    pub fn warm(&self, work: &Path) -> Result<Duration, ChromiumError> {
        let page = work.join("warm.html");
        std::fs::write(&page, WARM_PAGE)
            .map_err(|e| ChromiumError::Failed(format!("{}: {e}", page.display())))?;
        let started = Instant::now();
        let cold = Chromium {
            binary: self.binary.clone(),
            deadline: COLD_START,
        };
        cold.screenshot(&page, &work.join("warm.png"), 640, 400, 1)?;
        Ok(started.elapsed())
    }
}
