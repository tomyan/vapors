//! The bytes of the instanced pipelines (`instanced.rs`): an instance's
//! floats, then its row's index in its batch's range, its frame's entry
//! in the transform table and its clip's in the clip table (`table.rs`,
//! `clip_table.rs`), padded to 16 bytes; and the view uniform's bytes.

use vapors_scene::{ROWS_PER_BATCH, Size};

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
/// frame's entry in the table, and its clip's entry in the clip table
/// from `clips` ([`clip_of`]; 0 for none), padded to 16 bytes. An empty
/// `rows` puts every item in row 0 (a batch in one frame).
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
    (items, (frame, clips), rows): (&[T], (u32, ClipRuns<'_>), &[u32]),
    floats: impl Fn(&T) -> Vec<f32>,
) -> Vec<u8> {
    assert_rows(label, items.len(), rows);
    assert_clip_runs(label, items.len(), clips);
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
        bytes.extend(clip_of(clips, at).to_le_bytes());
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

/// The clip entries of a batch's items, as runs `(first item, entry)`
/// ([`clip_of`]).
pub type ClipRuns<'a> = &'a [(u32, u32)];

/// Item `at`'s clip entry in `clips`, runs of items under one clip as
/// `(first item, entry)` in item order: the entry of the last run that
/// starts at or before `at`; 0, no clip, before the first or with none.
/// A batch joined across clips names several (flat-scene F3.1 step 2).
#[must_use]
pub fn clip_of(clips: &[(u32, u32)], at: usize) -> u32 {
    let after = clips.partition_point(|&(first, _)| first as usize <= at);
    after.checked_sub(1).map_or(0, |run| clips[run].1)
}

/// Refuses, naming `label`, clip runs whose first items do not rise or
/// run past `items`: out of order, an item would read another's clip.
///
/// # Panics
///
/// As said.
fn assert_clip_runs(label: &str, items: usize, clips: &[(u32, u32)]) {
    let rising = clips.windows(2).all(|pair| pair[0].0 < pair[1].0);
    let inside = clips
        .last()
        .is_none_or(|&(first, _)| (first as usize) < items.max(1));
    assert!(
        rising && inside,
        "{label}: clip runs {clips:?} for {items} items: each run's first item must rise and \
         fall inside the batch (instanced_bytes.rs clip_of); reproduce with `cargo xtask test`"
    );
}

/// Refuses, naming `label`, rows that are not empty or one index for
/// each of `items` items, every one below the most rows a batch's range
/// holds ([`ROWS_PER_BATCH`], vapors-scene `batch.rs`).
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
    let past = rows.iter().position(|&row| row as usize >= ROWS_PER_BATCH);
    if let Some(at) = past {
        panic!(
            "{label}: item {at} names row {}, past the {} rows a batch's range holds; split the \
             run into batches of at most that many rows (vapors-scene batch.rs); reproduce with \
             `cargo xtask test`",
            rows[at], ROWS_PER_BATCH
        );
    }
}

#[cfg(test)]
#[path = "instanced_bytes_tests.rs"]
mod tests;
