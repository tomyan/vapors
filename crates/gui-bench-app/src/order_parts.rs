//! What an order row's classes are picked by, and the refusals when an
//! order has no class: the row's CSS has 4 pill classes, 8 avatar classes
//! and 7 bit segments for a 0..=100 fill (`FLAGS.md`).
use crate::dashboard::{AVATAR_COLORS, Order, Status};

/// The pill classes `s0`.. in `order_row.wisp`.
pub const PILLS: usize = 4;
/// The highest percentage the 7 bit segments of the fill can show.
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
        .expect("avatar colour not in AVATAR_COLORS: add its class to ui/order_row.wisp (cargo test -p gui-bench-app order_parts)");
    let status = pill(order.status.filter_row());
    let progress = order.progress;
    assert!(
        progress <= FULL,
        "progress {progress} is not a percentage: the fill has 7 bit segments, 0..=100 only (cargo test -p gui-bench-app order_parts)"
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
