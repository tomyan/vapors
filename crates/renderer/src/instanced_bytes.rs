//! The bytes of the instanced pipelines (`instanced.rs`): an instance's
//! floats, then its row's index in its batch's range, its frame's entry
//! in the transform table and its clip's in the clip table (`table.rs`,
//! `clip_table.rs`), padded to 16 bytes; and the view uniform's bytes.

use vapors_scene::Size;

use crate::{RowOffsets, Shift};

/// Bytes per instance of `vec4s` four-float attributes: the attributes and
/// the row index after them, padded to 16 bytes, as kept instances are
/// whole 16-byte rows (`retained.rs`).
pub(crate) const fn instance_bytes(vec4s: u32) -> usize {
    (vec4s as usize + 1) * 16
}

/// Little-endian bytes of `floats`: a pipeline's instances with no row
/// index (the mesh pipeline's and the scene's closed forms' own layouts,
/// `meshes.rs`, `closed_forms_3d.rs`).
pub(crate) fn float_bytes(floats: impl IntoIterator<Item = f32>) -> Vec<u8> {
    floats.into_iter().flat_map(f32::to_le_bytes).collect()
}

/// The viewport uniform's bytes for a pipeline that draws one item a
/// draw (images): its size and `shift`, two floats each; an item's row
/// offset is in its shift.
#[allow(clippy::cast_precision_loss)] // viewports are far below 2^24 pixels
pub(crate) fn viewport_bytes(viewport: Size, shift: Shift) -> [u8; 16] {
    let [dx, dy] = shift.pixels();
    let floats = [viewport.width as f32, viewport.height as f32, dx, dy];
    let mut bytes = [0; 16];
    for (at, float) in floats.into_iter().enumerate() {
        bytes[at * 4..at * 4 + 4].copy_from_slice(&float.to_le_bytes());
    }
    bytes
}

/// The view uniform's size: [`view_bytes`].
pub(crate) const VIEW_SIZE: usize = 16;

/// The view uniform's bytes: the viewport's size, two floats, then the
/// first entry of the batch's row range in the transform table.
#[allow(clippy::cast_precision_loss)] // viewports are far below 2^24 pixels
pub(crate) fn view_bytes(viewport: Size, rows_base: u32) -> [u8; VIEW_SIZE] {
    let mut bytes = [0; VIEW_SIZE];
    bytes[..4].copy_from_slice(&(viewport.width as f32).to_le_bytes());
    bytes[4..8].copy_from_slice(&(viewport.height as f32).to_le_bytes());
    bytes[8..12].copy_from_slice(&rows_base.to_le_bytes());
    bytes
}

/// The instance bytes of `items` for the pipeline `label`: each item's
/// `vec4s` four-float attributes (`floats`), then its row's index in
/// `rows` (in its batch's range of the transform table), `frame`, its
/// frame's entry in the table, and `clip`, its clip's entry in the clip
/// table (0 for none), padded to 16 bytes. An empty `rows` puts
/// every item in row 0 (a batch in one frame).
///
/// # Panics
///
/// When `rows` is neither empty nor one index an item, when an index is
/// past the most rows a batch's range holds ([`assert_rows`]), when an
/// item's floats are not `vec4s` four-float attributes, or when the bytes
/// built are not [`instance_bytes`] an item, the stride the pipeline
/// reads them at.
pub(crate) fn rowed_instances<T>(
    (label, vec4s): (&str, usize),
    (items, (frame, clip), rows): (&[T], (u32, u32), &[u32]),
    floats: impl Fn(&T) -> Vec<f32>,
) -> Vec<u8> {
    assert_rows(label, items.len(), rows);
    let mut bytes = Vec::new();
    for (at, item) in items.iter().enumerate() {
        let floats = floats(item);
        assert_eq!(
            floats.len(),
            vec4s * 4,
            "{label}: an instance is {vec4s} vec4s"
        );
        bytes.extend(floats.into_iter().flat_map(f32::to_le_bytes));
        bytes.extend(rows.get(at).copied().unwrap_or(0).to_le_bytes());
        bytes.extend(frame.to_le_bytes());
        bytes.extend(clip.to_le_bytes());
        bytes.extend([0; 4]);
    }
    #[allow(clippy::cast_possible_truncation)] // a handful of vec4s
    let instance = instance_bytes(vec4s as u32);
    assert_eq!(
        bytes.len(),
        items.len() * instance,
        "{label}: instances built at other than the {instance} bytes the pipeline reads \
         (instanced.rs `instance_bytes`)"
    );
    bytes
}

/// Refuses, naming `label`, rows that are not empty or one index for
/// each of `items` items, every one below the most rows a batch's range
/// holds ([`RowOffsets::CAPACITY`]).
///
/// # Panics
///
/// As said: a short `rows` would put the rest in row 0. An index past
/// its own batch's range, though under the cap, would read the
/// transform table's next entries; the painter refuses that where it
/// knows the range (vapors-app `painter_kept_bytes.rs` `rows_in_range`).
pub fn assert_rows(label: &str, items: usize, rows: &[u32]) {
    assert!(
        rows.is_empty() || rows.len() == items,
        "{label}: {} row indexes for {items} items; give one an item, or none for a batch in one \
         frame (instanced.rs); reproduce with `cargo xtask test`",
        rows.len()
    );
    let past = rows
        .iter()
        .position(|&row| row as usize >= RowOffsets::CAPACITY);
    if let Some(at) = past {
        panic!(
            "{label}: item {at} names row {}, past the {} rows a batch's table holds; split the \
             run into batches of at most that many rows (row_offsets.rs); reproduce with \
             `cargo xtask test`",
            rows[at],
            RowOffsets::CAPACITY
        );
    }
}
