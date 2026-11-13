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
/// kept in the atlas until given back with [`GlyphAtlas::unpin`], and
/// the atlas generation those bytes were built at.
///
/// A debug build panics when pins are dropped without being given back:
/// their entries would stay pinned until the next clear of every pin.
#[derive(Debug)]
#[must_use = "pinned glyphs stay in the atlas until unpinned"]
pub struct AtlasPins {
    keys: Vec<AtlasKey>,
    /// The pin epoch they were taken in: void once it moves on.
    epoch: u64,
    /// The atlas generation when the build that took them ended.
    pub(crate) generation: u64,
}

impl Drop for AtlasPins {
    fn drop(&mut self) {
        if cfg!(debug_assertions) && !self.keys.is_empty() && !std::thread::panicking() {
            panic!(
                "{} atlas pins dropped without GlyphAtlas::unpin: their glyphs stay pinned until \
                 the next clear of every pin; give every AtlasPins back (atlas_pins.rs); \
                 reproduce with `cargo xtask test`",
                self.keys.len()
            );
        }
    }
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

    /// The atlas generation the bytes built with these pins were built
    /// at: they are current while [`GlyphAtlas::generation`] equals it.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }
}

impl GlyphAtlas {
    /// Empty pins for the current epoch.
    pub fn new_pins(&self) -> AtlasPins {
        AtlasPins {
            keys: Vec::new(),
            epoch: self.pin_epoch,
            generation: self.generation,
        }
    }

    /// Moves on with every repack and pin clear: bytes built at an
    /// earlier generation may name moved or evicted glyphs.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// How many times every pin was cleared so far.
    #[must_use]
    pub fn pin_clears(&self) -> u64 {
        self.pin_epoch
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
    pub fn unpin(&mut self, mut pins: AtlasPins) {
        let keys = std::mem::take(&mut pins.keys);
        if pins.epoch != self.pin_epoch {
            return;
        }
        for key in keys {
            let entry = self.entries.get_mut(&key);
            let pins = entry.and_then(|entry| entry.pins.checked_sub(1).map(|n| (entry, n)));
            let (entry, left) = pins.unwrap_or_else(|| panic!("{}", unpinned_twice(&key)));
            entry.pins = left;
        }
    }

    /// Pins the entry under `key` into `pins`, when given. Pins made
    /// before a clear and still empty take the current epoch: a build's
    /// first pass may clear every pin before its second pass pins.
    ///
    /// # Panics
    ///
    /// When `pins` holds pins taken before the last clear.
    pub(crate) fn pin(&mut self, key: &AtlasKey, pins: Option<&mut AtlasPins>) {
        let Some(pins) = pins else { return };
        if pins.epoch != self.pin_epoch {
            assert!(
                pins.keys.is_empty(),
                "pins taken before every pin was cleared were added to (atlas_pins.rs): take a \
                 batch's pins in one pass; reproduce with `cargo xtask test`"
            );
            pins.epoch = self.pin_epoch;
        }
        let entry = self.entries.get_mut(key).unwrap_or_else(|| {
            panic!(
                "atlas entry {key:?} pinned but not in the atlas: pin only right after a get or \
                 an insert of it (atlas_pins.rs); reproduce with `cargo xtask test`"
            )
        });
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
