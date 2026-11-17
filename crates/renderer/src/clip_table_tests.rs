//! A clip entry's bytes as the shader reads them, the poison a given-back
//! entry takes, the chain's bound in the prelude, and the refusal when
//! the table cannot grow.

use super::{ClipEntry, clip_prelude};
use crate::table_entries::{Entries, EntryValue};
use crate::{ArenaFull, Holding};

#[test]
fn an_entry_is_48_bytes_rect_radii_frame_parent_flags() {
    // Given
    let entry = ClipEntry {
        rect: [1.0, 2.0, 3.0, 4.0],
        radii: [5.0, 6.0, 7.0, 8.0],
        frame: 9,
        parent: 10,
        row_relative: true,
        given_back: true,
    };

    // When
    let mut bytes = Vec::new();
    entry.write_bytes(&mut bytes);

    // Then
    assert_eq!(bytes.len() as u64, ClipEntry::BYTES);
    let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().expect("4"));
    assert_eq!(f32::from_bits(word(0)), 1.0);
    assert_eq!(f32::from_bits(word(28)), 8.0);
    assert_eq!((word(32), word(36), word(40), word(44)), (9, 10, 3, 0));
}

#[test]
fn a_given_back_entry_holds_the_poison_until_written() {
    // Given
    let mut entries = Entries::<ClipEntry>::new(4);
    let at = entries.take(1).expect("room");
    entries.set(at, &[ClipEntry::rect([0.0, 0.0, 8.0, 8.0], 0)], 1);

    // When
    entries.give(at, 1);

    // Then
    assert!(entries.get(at).given_back);
    assert!(entries.get(at).rect.iter().all(|v| v.is_nan()));
    assert_eq!(entries.take_poisoned(), Some((at, at)));
    assert_eq!(entries.take_poisoned(), None);
}

#[test]
fn the_prelude_carries_the_chain_s_bound() {
    assert!(clip_prelude().contains(&format!(
        "const MAX_CLIP_CHAIN: u32 = {}u;",
        vapors_scene::MAX_CLIP_CHAIN
    )));
}

#[test]
fn a_full_clip_table_names_the_limit_and_a_fix_for_clips() {
    // Given
    let full = ArenaFull {
        pipeline: "painter clips",
        size: 1 << 28,
        limit: 1 << 27,
        holding: Holding::ClipEntries {
            limit: "max_storage_buffer_binding_size",
        },
    };

    // Then
    let message = full.to_string();
    assert!(
        message.contains("the clip table's entries need"),
        "{message}"
    );
    assert!(message.contains("draw fewer clipped boxes"), "{message}");
}
