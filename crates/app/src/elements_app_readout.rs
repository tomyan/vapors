//! The GPU-time readout for a run by hand (`docs/plans/flat-scene-f7.md`,
//! F7b): once a second, the mean and the median GPU time of the frames
//! read back in it, the passes a frame's reading sums and the step its
//! timestamps resolve to, with the readings missing and why; short
//! enough for the demo bar's line on a phone (`report_readout`). Or,
//! once, that the device has no timestamp-query and so no GPU time. The
//! first frame says it is waiting, so a page that goes idle before a
//! reading arrives still shows the readout is on: an idle page draws no
//! frames, so no line follows until it draws again.

use std::time::Duration;

use vapors_platform::Instant;
use vapors_renderer::{GpuMissing, GpuReading, GpuResolution, GpuTime, Samples};

/// How often a line is written.
const EVERY: Duration = Duration::from_secs(1);

pub(super) struct GpuReadout {
    samples: Samples,
    /// The GPU times read back since the last line.
    times: Vec<Duration>,
    /// Readings since the last line with no time.
    missing: usize,
    /// The last reading's passes and step.
    shape: Option<(u32, GpuResolution)>,
    since: Option<Instant>,
    /// The last frame read back, so one reading is taken once.
    last: Option<u64>,
    said_unsupported: bool,
    said_waiting: bool,
    said_missing: bool,
}

impl GpuReadout {
    pub(super) fn new(samples: Samples) -> Self {
        Self {
            samples,
            times: Vec::new(),
            missing: 0,
            shape: None,
            since: None,
            last: None,
            said_unsupported: false,
            said_waiting: false,
            said_missing: false,
        }
    }

    /// Takes the app's GPU time after a frame at `now`; the line to write,
    /// when one is due.
    pub(super) fn take(&mut self, gpu: &GpuTime, now: Instant) -> Option<String> {
        self.since.get_or_insert(now);
        let samples = self.samples.count();
        match gpu {
            GpuTime::Unsupported if !self.said_unsupported => {
                (self.said_unsupported, self.said_waiting) = (true, true);
                return Some(format!("{samples}x no GPU time (no timestamp-query)"));
            }
            GpuTime::Read(reading) if self.last != Some(reading.frame) => {
                self.last = Some(reading.frame);
                if let Some(why) = self.record(reading) {
                    // it shows the readout is on, as the waiting line does
                    self.said_waiting = true;
                    return Some(format!("{samples}x GPU reading missing: {why}"));
                }
            }
            _ => {}
        }
        if !self.said_waiting {
            self.said_waiting = true;
            return Some(format!("{samples}x waiting for GPU time (none while idle)"));
        }
        self.line_due(now)
    }

    /// Counts `reading`; the reason of the first one missing.
    fn record(&mut self, reading: &GpuReading) -> Option<GpuMissing> {
        self.shape = Some((reading.pairs, reading.resolution));
        match &reading.time {
            Ok(time) => {
                self.times.push(*time);
                None
            }
            Err(why) => {
                self.missing += 1;
                (!std::mem::replace(&mut self.said_missing, true)).then(|| why.clone())
            }
        }
    }

    /// The second's line, once a second with a reading in it.
    fn line_due(&mut self, now: Instant) -> Option<String> {
        let since = self.since.unwrap_or(now);
        let read = self.times.len();
        if now.saturating_duration_since(since) < EVERY || read + self.missing == 0 {
            return None;
        }
        self.since = Some(now);
        let missing = std::mem::take(&mut self.missing);
        let mut times = std::mem::take(&mut self.times);
        let samples = self.samples.count();
        let gaps = (missing > 0).then(|| format!(", {missing} of {} missing", read + missing));
        let gaps = gaps.unwrap_or_default();
        if times.is_empty() {
            return Some(format!("{samples}x GPU none read{gaps}"));
        }
        times.sort_unstable();
        let median = times[times.len() / 2].as_micros();
        let mean = times.iter().sum::<Duration>().as_micros() / times.len() as u128;
        let shape = self.shape.map(shape_of).unwrap_or_default();
        Some(format!(
            "{samples}x GPU mean {mean}, median {median} µs{shape}{gaps}"
        ))
    }
}

/// `; N passes, <step>`: what a reading sums, and how finely.
fn shape_of((pairs, resolution): (u32, GpuResolution)) -> String {
    let step = match resolution {
        GpuResolution::Tick(tick) => format!("{} ns ticks", tick.as_nanos()),
        GpuResolution::BrowserQuantised => "browser-rounded".to_owned(),
    };
    format!("; {pairs} passes, {step}")
}

#[cfg(test)]
#[path = "elements_app_readout_tests.rs"]
mod tests;
