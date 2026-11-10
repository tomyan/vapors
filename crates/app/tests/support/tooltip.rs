//! Shared by the `gpu_tooltip*` tests: a tooltip style that fits the
//! 32px target, and where a colour shows in a frame.

// each test crate including this uses a part of it
#![allow(dead_code)]

use super::support::{SIZE, pixel};

pub const GREEN: [u8; 4] = [0, 255, 0, 255];

/// A green tooltip with 2px padding and square corners, its text blue
/// at 8px, so it fits the 32px target.
pub const TOOLTIP: &str = ":root { --tooltip-background: #00ff00; --tooltip-color: #0000ff; \
    --tooltip-font-size: 8px; --tooltip-padding: 2px; --tooltip-border-width: 0; \
    --tooltip-border-radius: 0; }";

/// The smallest rectangle `[left, top, right, bottom]` (inclusive)
/// holding every pixel of `rgba` equal to `colour`.
pub fn bounds(rgba: &[u8], colour: [u8; 4]) -> Option<[u32; 4]> {
    let at = |i: u32| (i % SIZE, i / SIZE);
    let found = (0..SIZE * SIZE).filter(|i| pixel(rgba, at(*i).0, at(*i).1) == colour);
    found.fold(None, |acc, i| {
        let (x, y) = at(i);
        Some(acc.map_or([x, y, x, y], |[l, t, r, b]: [u32; 4]| {
            [l.min(x), t.min(y), r.max(x), b.max(y)]
        }))
    })
}
