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

/// Each cube's `color` in the page's CSS (`.one`, `.two`, `.three`),
/// as sRGB bytes. The page's sheet and these must agree: `confirm`
/// compares what is drawn with them, so a disagreement fails it.
pub const CUBE_COLOURS: [[u8; 3]; CUBES] =
    [[0xc0, 0x39, 0x2b], [0x27, 0xae, 0x60], [0x29, 0x80, 0xb9]];

/// A lit cube's `color` (`.lit`).
pub const LIT: [u8; 3] = [0xf1, 0xc4, 0x0f];

/// The ground's `color`, the `--tint` token by day and by night.
pub const GROUND: [u8; 3] = [0x3b, 0x4a, 0x3f];
/// See [`GROUND`].
pub const GROUND_AT_NIGHT: [u8; 3] = [0x1d, 0x23, 0x40];

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

impl Expected {
    /// The ground's colour.
    #[must_use]
    pub fn ground(&self) -> [u8; 3] {
        if self.night { GROUND_AT_NIGHT } else { GROUND }
    }

    /// Cube `cube`'s colour and whether it is shown.
    #[must_use]
    pub fn cube(&self, cube: usize) -> ([u8; 3], bool) {
        let colour = if self.lit[cube] {
            LIT
        } else {
            CUBE_COLOURS[cube]
        };
        (colour, !(cube == CUBES - 1 && self.gone))
    }
}

/// The scripted changes due at `frame`, named for a report: a click,
/// and each class the time signal flips.
#[must_use]
pub fn changes_at(frame: usize) -> Vec<String> {
    let click = click_at(frame).map(|light| format!("click light {}", light + 1));
    let Some(before) = frame.checked_sub(1).map(expected) else {
        return click.into_iter().collect();
    };
    let now = expected(frame);
    let flips = [
        (before.gone != now.gone)
            .then(|| format!("cube 3 {}", if now.gone { "hidden" } else { "shown" })),
        (before.night != now.night)
            .then(|| format!("night {}", if now.night { "on" } else { "off" })),
    ];
    click
        .into_iter()
        .chain(flips.into_iter().flatten())
        .collect()
}

#[cfg(test)]
#[path = "schedule_tests.rs"]
mod tests;
