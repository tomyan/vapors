//! What differs between two oracle snapshots (`oracle.rs`).

use super::oracle::Snapshot;

/// The first differing entry of each part, so a failure on thousands of
/// rows names what differs rather than printing both snapshots.
pub(super) fn first_difference(kept: &Snapshot, fresh: &Snapshot) -> String {
    [
        differ("boxes", &kept.boxes, &fresh.boxes),
        differ("styles", &kept.styles, &fresh.styles),
        differ("items", &kept.items, &fresh.items),
        differ("clips", &kept.clips, &fresh.clips),
        differ("hits", &kept.hits, &fresh.hits),
        differ("focus", &[kept.focus], &[fresh.focus]),
        differ("selections", &kept.selections, &fresh.selections),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("\n")
}

fn differ<T: PartialEq + std::fmt::Debug>(part: &str, kept: &[T], fresh: &[T]) -> Option<String> {
    if kept.len() != fresh.len() {
        return Some(format!(
            "{part}: {} entries, {} full",
            kept.len(),
            fresh.len()
        ));
    }
    let at = kept.iter().zip(fresh).position(|(k, f)| k != f)?;
    Some(format!(
        "{part}[{at}]: {:?}\n  full: {:?}",
        kept[at], fresh[at]
    ))
}
