//! Pins (IU4 PR 3, `docs/plans/iu4.md` section 2): kept text and icon
//! bytes name atlas rects that no frame fetches again, so the LRU would
//! take them for unused. Building kept bytes pins each glyph they name;
//! releasing them unpins. Eviction skips pinned glyphs and panics when
//! asked to take one. When nothing unpinned can make room and the page
//! cannot grow, every pin is cleared at once and the generation moves
//! on, so every kept batch is built again on its next draw; pins taken
//! before the clear are then void. Every repack moves the generation on
//! too: it moves glyphs, and with them the rects kept bytes name.

use crate::atlas::GlyphAtlas;
use crate::atlas_types::AtlasKey;

/// The atlas entries one kept batch's bytes name, each once per use:
/// kept in the atlas until given back with [`GlyphAtlas::unpin`].
#[derive(Debug)]
#[must_use = "pinned glyphs stay in the atlas until unpinned"]
pub struct AtlasPins {
    keys: Vec<AtlasKey>,
    /// The pin epoch they were taken in: void once it moves on.
    epoch: u64,
}

impl AtlasPins {
    /// How many pins: one per use of an entry.
    #[must_use]
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// True when none were taken.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}

impl GlyphAtlas {
    /// Empty pins for the current epoch.
    pub fn new_pins(&self) -> AtlasPins {
        AtlasPins {
            keys: Vec::new(),
            epoch: self.pin_epoch,
        }
    }

    /// Moves on with every repack and pin clear: bytes built at an
    /// earlier generation may name moved or evicted glyphs.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Cache lookups so far, by glyph or icon.
    #[must_use]
    pub fn lookups(&self) -> u64 {
        self.lookups
    }

    /// Gives `pins` back; nothing when they were taken before the last
    /// clear of every pin.
    ///
    /// # Panics
    ///
    /// When a pinned entry is gone or holds fewer pins than given back:
    /// pinned entries are never evicted, so the pins were given twice.
    pub fn unpin(&mut self, pins: AtlasPins) {
        if pins.epoch != self.pin_epoch {
            return;
        }
        for key in pins.keys {
            let entry = self.entries.get_mut(&key);
            let pins = entry.and_then(|entry| entry.pins.checked_sub(1).map(|n| (entry, n)));
            let (entry, left) = pins.unwrap_or_else(|| panic!("{}", unpinned_twice(&key)));
            entry.pins = left;
        }
    }

    /// Pins the entry under `key` into `pins`, when given.
    pub(crate) fn pin(&mut self, key: &AtlasKey, pins: Option<&mut AtlasPins>) {
        let Some(pins) = pins else { return };
        let entry = self.entries.get_mut(key).expect("pinned right after a get");
        entry.pins += 1;
        pins.keys.push(key.clone());
    }

    /// Clears every pin, voiding those taken, and moves the generation
    /// on; false when nothing was pinned.
    pub(crate) fn unpin_all(&mut self) -> bool {
        let mut any = false;
        for entry in self.entries.values_mut().filter(|entry| entry.pins > 0) {
            entry.pins = 0;
            any = true;
        }
        if any {
            self.pin_epoch += 1;
            self.generation += 1;
        }
        any
    }

    /// Removes the entry under `key`.
    ///
    /// # Panics
    ///
    /// When it is pinned: kept bytes would name a rect another glyph
    /// takes.
    pub(crate) fn evict(&mut self, key: &AtlasKey) {
        let pins = self.entries.get(key).map_or(0, |entry| entry.pins);
        assert!(
            pins == 0,
            "evicting atlas entry {key:?}, pinned {pins} time(s) by kept bytes; eviction must skip \
             pinned entries (atlas_evict.rs); reproduce with `cargo xtask test`"
        );
        self.entries.remove(key);
    }
}

fn unpinned_twice(key: &AtlasKey) -> String {
    format!(
        "atlas entry {key:?} unpinned more times than it was pinned, or gone while pinned; give \
         each AtlasPins back once (atlas_pins.rs); reproduce with `cargo xtask test`"
    )
}

#[cfg(test)]
#[path = "atlas_pins_tests.rs"]
mod tests;
