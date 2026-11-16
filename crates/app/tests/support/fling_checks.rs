//! What a replayed coast (`fling_replay.rs`) is held to beside
//! Chrome's: its path, where it comes to rest, the list's edge, and the
//! app going idle after.

use vapors_platform::Next;

use super::fling_replay::{Compared, Replayed};

/// The coast's first frames, held to `FIRST_ALLOWANCE`; the fit saw
/// these and nothing after them (`fit.py coast`).
pub const FIRST_FRAMES: usize = 12;
/// How far vapors' offset may be from Chrome's over the first frames,
/// CSS px: the fit saw these frames, and on each fixture it came within
/// 2.1 to 4.6 px of them (frames quantised to device pixels, the start
/// time fitted to the half millisecond).
pub const FIRST_ALLOWANCE: f32 = 6.0;
/// After the first frames, how far vapors' offset may be from Chrome's
/// as a share of the distance Chrome's coast travelled. Measured out of
/// sample (the fit saw only the first frames): the worst whole-coast
/// error over distance travelled across the nine fixtures was 0.0113
/// (003, 16.75 px over 1485 px; the plan lists each), rounded up. It is
/// recording noise and the start fitted on 12 frames, not slack: a curve
/// constant 10% off fails it (the commit says which breaks were run).
pub const LATE_RATIO: f32 = 0.012;
/// Closer to its final offset than this, a list is at rest, CSS px.
const REST: f32 = 0.5;
/// How many frames apart vapors and Chrome may come to rest. Chrome's
/// last moving frame moves 0.4 to 0.7 px, past where its fitted curve
/// ends. Retires with #395 (Chrome's fling resampling), if that explains
/// the tail.
pub const REST_FRAMES_APART: usize = 1;
/// Frames Chrome's list holds still at the end of a recording that came
/// to rest: the recorder saves 500 ms after the list last moved, 30
/// frames; a coast cut by the next touch ends moving.
const STILL_AT_REST: usize = 5;
/// Frames after vapors' offset last changes by which the app asks for
/// no more: the fling ends within 0.1 px of its curve's end, and the
/// frame after the last change can still be running.
pub const IDLE_WITHIN: usize = 2;

/// The worst distance from Chrome over the first frames and the whole
/// coast, CSS px.
pub fn worst(compared: &[Compared]) -> (f32, f32) {
    let off = |frames: &[Compared]| {
        frames
            .iter()
            .map(|f| (f.ours - f.chrome).abs())
            .fold(0.0, f32::max)
    };
    (
        off(&compared[..FIRST_FRAMES.min(compared.len())]),
        off(compared),
    )
}

/// How far Chrome's list moved over the recorded coast, CSS px.
pub fn travelled(compared: &[Compared]) -> f32 {
    match (compared.first(), compared.last()) {
        (Some(first), Some(last)) => (last.chrome - first.chrome).abs(),
        _ => 0.0,
    }
}

/// Whether Chrome's list held still at the end of the recording.
pub fn came_to_rest(compared: &[Compared]) -> bool {
    let tail = &compared[compared.len().saturating_sub(STILL_AT_REST + 1)..];
    tail.windows(2)
        .all(|pair| pair[0].chrome.to_bits() == pair[1].chrome.to_bits())
}

/// The last frame at which the offset `pick` reads is not yet at rest.
fn last_moving(compared: &[Compared], pick: fn(&Compared) -> f32) -> Option<usize> {
    let end = compared.last().map_or(0.0, pick);
    compared
        .iter()
        .rposition(|frame| (pick(frame) - end).abs() > REST)
}

/// How many frames after Chrome's list came to rest vapors' did
/// (negative: before).
pub fn rest_gap(compared: &[Compared]) -> Option<isize> {
    let signed = |n: usize| isize::try_from(n).expect("a few frames");
    let ours = last_moving(compared, |f| f.ours)?;
    let chrome = last_moving(compared, |f| f.chrome)?;
    Some(signed(ours) - signed(chrome))
}

/// Frames off Chrome's path by more than their allowance, as lines.
pub fn off_path(file: &str, compared: &[Compared]) -> Vec<String> {
    let late = LATE_RATIO * travelled(compared);
    let allowance = |at: usize| {
        if at < FIRST_FRAMES {
            FIRST_ALLOWANCE
        } else {
            late
        }
    };
    compared
        .iter()
        .enumerate()
        .filter(|(at, frame)| {
            (frame.ours - frame.chrome).abs() > allowance(*at) || frame.ours.is_nan()
        })
        .map(|(at, f)| {
            format!(
                "{file} frame {at} at {:.1} ms: ours {:.1}, Chrome's {:.1}",
                f.t, f.ours, f.chrome
            )
        })
        .collect()
}

/// Where a coast that came to rest rests apart from Chrome's, as a line.
pub fn rest_apart(file: &str, compared: &[Compared]) -> Option<String> {
    let gap = rest_gap(compared);
    let within = gap.is_some_and(|gap| gap.unsigned_abs() <= REST_FRAMES_APART);
    (!within).then(|| format!("{file}: came to rest {gap:?} frames from Chrome"))
}

/// Chrome's coast ending this close to an end of the list ended there,
/// CSS px: its `scrollTop` is quantised to device pixels (1/2.75 px).
const AT_EDGE: f32 = 1.0;

/// Where Chrome's coast stopped at an end of the list and vapors' did
/// not stop exactly there, as a line.
pub fn off_edge(file: &str, compared: &[Compared], max: f32) -> Option<String> {
    let last = compared.last()?;
    let edge = [0.0, max]
        .into_iter()
        .find(|edge| (last.chrome - edge).abs() <= AT_EDGE)?;
    #[allow(
        clippy::float_cmp,
        reason = "a scroll clamped to its end is exactly there"
    )]
    let at_edge = last.ours == edge;
    (!at_edge).then(|| {
        format!(
            "{file}: Chrome stopped at the edge {edge}, vapors at {:.2}",
            last.ours
        )
    })
}

/// Where the app kept asking for frames more than `IDLE_WITHIN` frames
/// after its offset last changed, or asked for none while it still
/// moved, as a line.
pub fn not_idle(file: &str, replayed: &Replayed) -> Option<String> {
    let frames = &replayed.frames;
    let changed = frames
        .windows(2)
        .rposition(|pair| pair[0].0.to_bits() != pair[1].0.to_bits());
    let last_change = changed.map_or(0, |at| at + 1);
    let idle = frames.iter().position(|(_, next)| *next == Next::Wait);
    let settled =
        idle.is_some_and(|idle| (last_change..=last_change + IDLE_WITHIN).contains(&idle));
    (!settled).then(|| {
        format!(
            "{file}: the offset last changed at frame {last_change}; the app went idle at {idle:?}"
        )
    })
}
