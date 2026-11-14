//! The scene's script on the frame clock: deterministic, from the frame's
//! index alone. Two kinds of change, as real pages make them: a click on
//! a HUD button every 2 s, cycling through the three lights, each
//! toggling its cube's `lit` class; and changes the page makes itself
//! from its time signal: the group turning every frame, the third cube
//! hidden in every other 5 s span, the night token on in every other
//! 15 s span.

use std::time::Duration;

/// The frame clock's step, as `vapors-perf`'s sampler advances it.
pub const FRAME: Duration = Duration::from_millis(16);

/// The run: 60 s of the frame clock.
pub const RUN_FRAMES: usize = 3750;
const _: () = assert!(RUN_FRAMES as u128 * FRAME.as_millis() == 60_000);

/// The cubes, and the HUD buttons that light them.
pub const CUBES: usize = 3;

/// Frames between clicks: 2 s.
const CLICK_EVERY: usize = 125;

/// The span the third cube is shown, then hidden, in turn.
const GONE_SPAN: Duration = Duration::from_secs(5);

/// The span the night token is off, then on, in turn.
const NIGHT_SPAN: Duration = Duration::from_secs(15);

/// How fast the group turns, in radians a second.
const TURN_RATE: f32 = 0.5;

/// What the scene shows at a frame, once its changes are made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Expected {
    /// Each cube's `lit` class.
    pub lit: [bool; CUBES],
    /// Whether the third cube is hidden.
    pub gone: bool,
    /// Whether the night token is on.
    pub night: bool,
}

/// The frame clock's time at `frame`.
///
/// # Panics
/// For a frame past `u32::MAX`, far beyond any run.
#[must_use]
pub fn time_of(frame: usize) -> Duration {
    FRAME * u32::try_from(frame).expect("a run is far under u32::MAX frames")
}

/// The light clicked before `frame`, if one is.
#[must_use]
pub fn click_at(frame: usize) -> Option<usize> {
    (frame > 0 && frame.is_multiple_of(CLICK_EVERY)).then(|| (frame / CLICK_EVERY - 1) % CUBES)
}

/// Whether the third cube is hidden at `time`.
#[must_use]
pub fn gone(time: Duration) -> bool {
    span_odd(time, GONE_SPAN)
}

/// Whether the night token is on at `time`.
#[must_use]
pub fn night(time: Duration) -> bool {
    span_odd(time, NIGHT_SPAN)
}

/// The group's turn about +Y at `time`, in radians.
#[must_use]
pub fn turn(time: Duration) -> f32 {
    time.as_secs_f32() * TURN_RATE
}

fn span_odd(time: Duration, span: Duration) -> bool {
    (time.as_millis() / span.as_millis()) % 2 == 1
}

/// What the scene shows at `frame`, its click included.
#[must_use]
pub fn expected(frame: usize) -> Expected {
    let mut lit = [false; CUBES];
    for light in (1..=frame).filter_map(click_at) {
        lit[light] = !lit[light];
    }
    let time = time_of(frame);
    Expected {
        lit,
        gone: gone(time),
        night: night(time),
    }
}

/// The scripted changes due at `frame`, named for a report: a click,
/// and each class the time signal flips.
#[must_use]
pub fn changes_at(frame: usize) -> Vec<String> {
    let mut changes: Vec<String> = click_at(frame)
        .map(|light| format!("click light {}", light + 1))
        .into_iter()
        .collect();
    let (now, before) = (expected(frame), frame.checked_sub(1).map(expected));
    if before.is_some_and(|before| before.gone != now.gone) {
        changes.push(format!(
            "cube 3 {}",
            if now.gone { "hidden" } else { "shown" }
        ));
    }
    if before.is_some_and(|before| before.night != now.night) {
        changes.push(format!("night {}", if now.night { "on" } else { "off" }));
    }
    changes
}

#[cfg(test)]
#[path = "schedule_tests.rs"]
mod tests;
