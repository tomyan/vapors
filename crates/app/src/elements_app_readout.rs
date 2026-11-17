//! The GPU-time readout for a run by hand (`docs/plans/flat-scene-f7.md`,
//! F7b): once a second, led by the mean GPU time of the frames read back
//! in it as a share of the frame interval measured over them (the median
//! gap between frames: 60, 90 or 120 Hz, not assumed), then the mean and
//! the median, the passes a frame's reading sums and the step its
//! timestamps resolve to, with the readings missing and why; it wraps in
//! the demo bar's line on a phone (`report_readout`). Or,
//! once, that the device has no timestamp-query and so no GPU time. The
//! first frame says it is waiting, so a page that goes idle before a
//! reading arrives still shows the readout is on: an idle page draws no
//! frames, so no line follows until it draws again.

use std::time::Duration;

use vapors_platform::Instant;
use vapors_renderer::{GpuMissing, GpuReading, GpuResolution, GpuTime, Samples};

/// How often a line is written.
const EVERY: Duration = Duration::from_secs(1);

/// The longest gap between frames that counts as a frame interval: 10 Hz,
/// slower than any display's refresh; a longer one is an idle page's wait
/// or a stall, not the rate the page runs at.
const MAX_INTERVAL: Duration = Duration::from_millis(100);

pub(super) struct GpuReadout {
    samples: Samples,
    /// The GPU times read back since the last line.
    times: Vec<Duration>,
    /// The gaps between the frames since the last line: the frame
    /// interval the page runs at (60, 90 or 120 Hz), measured.
    intervals: Vec<Duration>,
    /// The last frame's time, for the next gap.
    last_now: Option<Instant>,
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
            intervals: Vec::new(),
            last_now: None,
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
        if let Some(before) = self.last_now.replace(now) {
            let gap = now.saturating_duration_since(before);
            if gap <= MAX_INTERVAL {
                self.intervals.push(gap);
            }
        }
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
        let median = times[times.len() / 2];
        let mean = times.iter().sum::<Duration>() / u32::try_from(times.len()).unwrap_or(u32::MAX);
        let interval = median_of(std::mem::take(&mut self.intervals));
        let shape = self.shape.map(shape_of).unwrap_or_default();
        let (head, detail) = lead(mean, interval);
        Some(format!(
            "{samples}x GPU {head} ({detail}{} median{shape}){gaps}",
            ms(median)
        ))
    }
}

/// What leads the line and what its details start with: the mean GPU
/// time as a share of the measured frame interval ("23% of 8.3 ms
/// frame", then "1.9 ms mean, "); without an interval, the mean and that
/// the interval is not measured yet (no two frames came within
/// `MAX_INTERVAL` of each other), rather than a 60 Hz assumed.
fn lead(mean: Duration, interval: Option<Duration>) -> (String, String) {
    let Some(interval) = interval.filter(|interval| !interval.is_zero()) else {
        let head = format!("{} mean, frame interval not measured yet", ms(mean));
        return (head, String::new());
    };
    let share = (mean.as_secs_f64() / interval.as_secs_f64() * 100.0).round();
    let head = format!("{share:.0}% of {} frame", ms(interval));
    (head, format!("{} mean, ", ms(mean)))
}

/// `values`' median; `None` when there are none.
fn median_of(mut values: Vec<Duration>) -> Option<Duration> {
    values.sort_unstable();
    values.get(values.len() / 2).copied()
}

/// `time` in milliseconds to a tenth: "8.3 ms".
fn ms(time: Duration) -> String {
    format!("{:.1} ms", time.as_secs_f64() * 1000.0)
}

/// `; N passes, <step>`: what a reading sums, and how finely.
fn shape_of((pairs, resolution): (u32, GpuResolution)) -> String {
    let step = match resolution {
        GpuResolution::Tick(tick) => format!("{} ns ticks", tick.as_nanos()),
        GpuResolution::BrowserQuantised => "browser-rounded".to_owned(),
    };
    let passes = if pairs == 1 { "pass" } else { "passes" };
    format!("; {pairs} {passes}, {step}")
}

#[cfg(test)]
#[path = "elements_app_readout_tests.rs"]
mod tests;
