//! The two workloads, exactly. All sizes are px; leaves have fixed
//! boxes and no text, so every option lays out the same thing (text
//! shaping is a separate cost, not measured here).
//!
//! Table(n): a flex column of `n` rows, 1000 wide. A row is a flex row
//! `ROW_H` tall of five cells: widths `COLS` (none shrinks) then a fifth
//! that takes the rest (flex 1 1 0). The changed leaf is cell 2 of row
//! `n / 2`, `COLS[1]` or `CHANGED_W` wide. Probe: that cell and the
//! fifth cell of that row.
//!
//! Deep(d): `d` flex containers, level 0 the root (2000 by 2000), each
//! inside the last as its second child after a leaf. Even levels are
//! columns and odd rows; each has `PAD` padding. A level's leaf is
//! `LEAF` square, flex none; the nested container is flex 1 1 0 and
//! stretches. The last level holds its leaf only. The changed leaf is
//! that of level `d - 2`, `LEAF` or `CHANGED_LEAF` square. Probe: that
//! leaf and the last level's leaf.

use crate::Workload;

pub const VIEW_W: f32 = 1000.0;
pub const ROW_H: f32 = 24.0;
pub const COLS: [f32; 4] = [80.0, 120.0, 100.0, 140.0];
pub const CHANGED_W: f32 = 150.0;
pub const DEEP_SIDE: f32 = 2000.0;
pub const PAD: f32 = 2.0;
pub const LEAF: f32 = 10.0;
pub const CHANGED_LEAF: f32 = 14.0;

/// The rows and depths measured and tested.
pub const TABLE_SIZES: [usize; 3] = [30, 300, 3000];
pub const DEEP_DEPTH: usize = 100;

/// What the probe's two boxes are, for messages.
#[must_use]
pub fn probe_names(workload: Workload) -> [&'static str; 2] {
    match workload {
        Workload::Table(_) => ["middle row, cell 2", "middle row, cell 5"],
        Workload::Deep(_) => ["leaf of level d-2", "leaf of the last level"],
    }
}

/// The index of the middle row, whose cell 2 changes.
#[must_use]
pub fn middle(rows: usize) -> usize {
    rows / 2
}

/// Row `index`'s top edge.
///
/// # Panics
/// `index` is over 65535: no workload here has so many rows.
#[must_use]
pub fn row_y(index: usize) -> f32 {
    ROW_H * f32::from(u16::try_from(index).expect("a row index under 65536"))
}
