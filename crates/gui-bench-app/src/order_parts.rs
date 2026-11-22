//! What an order row's classes are picked by, and the refusals when an
//! order has no class or colour: the row has 4 pill classes, an avatar palette
//! and a fill bound to a 0..=100 percentage (`FLAGS.md`).
use crate::dashboard::{AVATAR_COLORS, Order, Status};

/// The pill classes `s0`.. in `order_row.wisp`.
pub const PILLS: usize = 4;
/// The highest percentage: the fill is that share of its 120 px track.
pub const FULL: u32 = 100;
const _: () = assert!(
    Status::ALL.len() == PILLS,
    "Status has a variant the pill CSS has no class for: add `sN` to ui/order_row.wisp and raise PILLS"
);

/// The avatar class, the pill class and the progress of `order`; refuses
/// an order the row's CSS cannot show.
pub fn parts(order: &Order) -> (usize, usize, u32) {
    let colour = AVATAR_COLORS
        .iter()
        .position(|c| *c == order.avatar)
        .expect("avatar colour not in AVATAR_COLORS: add it to the spec palette read by src/data_colours.rs (cargo test -p gui-bench-app order_parts)");
    let status = pill(order.status.filter_row());
    let progress = order.progress;
    assert!(
        progress <= FULL,
        "progress {progress} is not a percentage: the fill is a share of its track, 0..=100 only (cargo test -p gui-bench-app order_parts)"
    );
    (colour, status, progress)
}

/// The pill class for a checkbox row `index`; refuses one past the classes.
pub fn pill(index: usize) -> usize {
    assert!(
        index < PILLS,
        "status {index} has no pill class: add s{index} to ui/order_row.wisp (cargo test -p gui-bench-app order_parts)"
    );
    index
}

/// The fill's width in px for a percentage `parts` has accepted: its share
/// of the 120 px track.
pub fn fill_width(progress: u32) -> f32 {
    let percent = u8::try_from(progress).expect("a percentage from parts, 0..=100");
    f32::from(percent) * 1.2
}
