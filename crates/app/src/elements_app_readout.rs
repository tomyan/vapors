//! The GPU-time readout for a run by hand (`docs/plans/flat-scene-f7.md`,
//! F7b): once a second, the median GPU time of the frames read back in
//! it and the painter's sample count, short enough for the demo bar's
//! line on a phone (`report_readout`); or,
//! once, that the device has no timestamp-query and so no GPU time. The
//! first frame says it is waiting, so a page that goes idle before a
//! reading arrives still shows the readout is on.

use std::time::Duration;

use vapors_platform::Instant;
use vapors_renderer::{GpuTime, Samples};

/// How often a line is written.
const EVERY: Duration = Duration::from_secs(1);

pub(super) struct GpuReadout {
    samples: Samples,
    /// The GPU times read back since the last line.
    times: Vec<Duration>,
    since: Option<Instant>,
    /// The last frame read back, so one reading is taken once.
    last: Option<u64>,
    said_unsupported: bool,
    said_waiting: bool,
}

impl GpuReadout {
    pub(super) fn new(samples: Samples) -> Self {
        Self {
            samples,
            times: Vec::new(),
            since: None,
            last: None,
            said_unsupported: false,
            said_waiting: false,
        }
    }

    /// Takes the app's GPU time after a frame at `now`; the line to write,
    /// when one is due.
    pub(super) fn take(&mut self, gpu: &GpuTime, now: Instant) -> Option<String> {
        self.since.get_or_insert(now);
        match gpu {
            GpuTime::Unsupported if !self.said_unsupported => {
                (self.said_unsupported, self.said_waiting) = (true, true);
                return Some(format!(
                    "{}x no GPU time (no timestamp-query)",
                    self.samples.count()
                ));
            }
            GpuTime::Read(reading) if self.last != Some(reading.frame) => {
                self.last = Some(reading.frame);
                self.times.extend(reading.time.as_ref().ok());
            }
            _ => {}
        }
        if !self.said_waiting {
            self.said_waiting = true;
            let samples = self.samples.count();
            return Some(format!("{samples}x waiting for GPU time"));
        }
        self.line_due(now)
    }

    /// The median line, once a second with a time read in it.
    fn line_due(&mut self, now: Instant) -> Option<String> {
        let since = self.since.unwrap_or(now);
        if now.saturating_duration_since(since) < EVERY || self.times.is_empty() {
            return None;
        }
        self.since = Some(now);
        let mut times = std::mem::take(&mut self.times);
        times.sort_unstable();
        let median = times[times.len() / 2].as_micros();
        // the median; a browser rounds GPU timestamps to 100 µs
        // (gpu_time.rs), said in the run's report, not in the line
        let samples = self.samples.count();
        Some(format!("{samples}x GPU {median} µs"))
    }
}

#[cfg(test)]
#[path = "elements_app_readout_tests.rs"]
mod tests;
