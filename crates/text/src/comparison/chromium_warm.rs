//! The browser's first launch on a fresh machine, taken before the
//! screenshots whose deadline assumes a warm one.

use std::path::Path;
use std::time::{Duration, Instant};

use super::chromium::{Chromium, ChromiumError};

/// The first launch on a fresh runner: the 1x paint test, which starts
/// the job's first Chromium, took 15.4 s in run 40020237373 (its other
/// Chromium tests took 2.2 to 5.5 s); twice that passed the 20 s
/// screenshot deadline in runs 40018292931 and 40019496284. 45 s is
/// about three times the measurement.
const COLD_START: Duration = Duration::from_secs(45);

impl Chromium {
    /// One screenshot of a blank page in `work`, with the cold-start
    /// deadline, so later screenshots meet their warm one. Returns how
    /// long it took.
    ///
    /// # Errors
    ///
    /// [`ChromiumError::Failed`] when the page cannot be written, or the
    /// screenshot fails or passes the cold-start deadline.
    pub fn warm(&self, work: &Path) -> Result<Duration, ChromiumError> {
        let page = work.join("warm.html");
        std::fs::write(&page, "<!doctype html>")
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
