//! A transform table for the GPU tests (`transform_table.rs`): a frame
//! entry translated by a shift, and a range of rows each translated by
//! its offset.

// each test crate including this uses a part of it
#![allow(dead_code)]

use vapors_renderer::{Target, TransformTable, translation};

/// A table, its frame entry, and its rows' first entry (0, the identity,
/// for no rows).
pub struct Table {
    pub transforms: TransformTable,
    pub frame: u32,
    pub rows_base: u32,
}

/// A table whose frame entry is moved by `shift` and whose rows are at
/// `rows`, device pixels.
pub fn table(target: &Target<'_>, shift: [f32; 2], rows: &[[f32; 2]]) -> Table {
    let mut table = TransformTable::new(target.device, "test transforms");
    let frame = table.take(target.encoder, 1).expect("room");
    table.set(target.encoder, target.queue, frame, &[translation(shift)]);
    let rows_base = if rows.is_empty() {
        0
    } else {
        let count = u32::try_from(rows.len()).expect("a few rows");
        let base = table.take(target.encoder, count).expect("room");
        let values: Vec<_> = rows.iter().map(|&row| translation(row)).collect();
        table.set(target.encoder, target.queue, base, &values);
        base
    };
    Table {
        transforms: table,
        frame,
        rows_base,
    }
}
