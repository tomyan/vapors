//! One frame's work in design 02's order (2-S1): input, tasks, the
//! animation tick, the update, accessibility, draw, record. Every phase
//! is given the frame's time, read once when the frame starts, and each
//! is timed by a [`Clock`] and wrapped in a `tracing` span named for it.
//! A failed update or draw stops the frame; the phases after it do not
//! run, and the error names the phase.

use std::fmt;
use std::time::Duration;

use vapors_platform::Instant;

/// A phase of the frame, in the order they run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The events since the last frame. They are dispatched as they
    /// arrive (design 02, 2-S1), so this phase holds the time their
    /// handlers took.
    Input,
    /// Futures woken since the last frame (2-S2).
    Tasks,
    /// Running transitions sampled at the frame's time (2-S3).
    Tick,
    /// `Document::update`: restyle, layout, paint to a scene.
    Update,
    /// The accessibility tree's update (area 5).
    Accessibility,
    /// The scene submitted and presented.
    Draw,
    /// The frame's recording (area 10).
    Record,
}

impl Phase {
    /// The phase's name in a line of times: `input`, `update`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Phase::Input => "input",
            Phase::Tasks => "tasks",
            Phase::Tick => "tick",
            Phase::Update => "update",
            Phase::Accessibility => "accessibility",
            Phase::Draw => "draw",
            Phase::Record => "record",
        }
    }

    /// Every phase, in order.
    pub const ALL: [Phase; 7] = [
        Phase::Input,
        Phase::Tasks,
        Phase::Tick,
        Phase::Update,
        Phase::Accessibility,
        Phase::Draw,
        Phase::Record,
    ];
}

/// What tells the time: the system's, or a test's stepped by hand.
pub trait Clock {
    /// The time now.
    fn now(&self) -> Instant;
}

/// The system's monotonic clock (`performance.now()` on the web).
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// How long each phase of a frame took.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameTimes([Duration; 7]);

impl FrameTimes {
    /// The time `phase` took.
    #[must_use]
    pub fn of(&self, phase: Phase) -> Duration {
        self.0[phase as usize]
    }
}

/// A phase that refused, which stopped the frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameError {
    /// The phase that refused.
    pub phase: Phase,
    /// Its refusal.
    pub message: String,
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the frame stopped in {:?}: {}", self.phase, self.message)
    }
}

/// The work of each phase; those that are not built yet do nothing.
pub trait Phases {
    /// The time event handlers took since the last frame: events are
    /// dispatched as they arrive, so this is the Input phase's time
    /// beside what `input` itself takes.
    fn input_time(&mut self) -> Duration {
        Duration::ZERO
    }
    /// Input work in the frame itself (none yet).
    fn input(&mut self, _now: Instant) {}
    /// Woken futures (2-S2).
    fn tasks(&mut self, _now: Instant) {}
    /// Running transitions sampled (2-S3).
    fn tick(&mut self, _now: Instant) {}
    /// The document's update.
    ///
    /// # Errors
    /// The update's refusal, which stops the frame.
    fn update(&mut self, now: Instant) -> Result<(), String>;
    /// The accessibility tree's update (area 5).
    fn accessibility(&mut self, _now: Instant) {}
    /// The scene drawn.
    ///
    /// # Errors
    /// The draw's failure, which stops the frame.
    fn draw(&mut self, now: Instant) -> Result<(), String>;
    /// Given the times of the phases before it.
    fn record(&mut self, _now: Instant, _times: &FrameTimes) {}
}

/// Runs one frame at `now`, timing each phase with `clock`.
///
/// # Errors
/// The first phase that refused, naming it; the phases after it do not run.
pub fn run_frame(
    phases: &mut impl Phases,
    now: Instant,
    clock: &impl Clock,
) -> Result<FrameTimes, FrameError> {
    let mut times = FrameTimes::default();
    for phase in Phase::ALL {
        let _span = tracing::trace_span!("frame phase", ?phase).entered();
        let started = clock.now();
        let result = run_phase(phases, phase, now, &times);
        times.0[phase as usize] = clock.now().saturating_duration_since(started);
        if phase == Phase::Input {
            times.0[0] += phases.input_time();
        }
        result.map_err(|message| FrameError { phase, message })?;
    }
    Ok(times)
}

fn run_phase(
    phases: &mut impl Phases,
    phase: Phase,
    now: Instant,
    times: &FrameTimes,
) -> Result<(), String> {
    match phase {
        Phase::Input => phases.input(now),
        Phase::Tasks => phases.tasks(now),
        Phase::Tick => phases.tick(now),
        Phase::Update => return phases.update(now),
        Phase::Accessibility => phases.accessibility(now),
        Phase::Draw => return phases.draw(now),
        Phase::Record => phases.record(now, times),
    }
    Ok(())
}

#[cfg(test)]
#[path = "frame/tests.rs"]
mod tests;
