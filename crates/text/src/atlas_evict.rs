//! Making room in a full page: evict the least recently used glyphs that
//! the current frame has not asked for and kept bytes have not pinned,
//! repack the survivors in a fixed order, and try again until the new
//! glyph fits. When every glyph left belongs to the current frame or is
//! pinned, grow the page (`atlas_grow.rs`); when it cannot grow, clear
//! every pin (`atlas_pins.rs`) and evict again.

use crate::atlas::{Entry, GlyphAtlas, glyph_at};
use crate::atlas_types::{AtlasError, AtlasKey};

impl GlyphAtlas {
    /// Room for a `width` by `height` glyph, or [`AtlasError::Full`] when
    /// this frame's glyphs do not fit the largest page. Pinned glyphs
    /// are kept while the page can grow; past that, every pin is cleared
    /// (`atlas_pins.rs`) and eviction tried again.
    pub(crate) fn make_room(&mut self, width: u32, height: u32) -> Result<[u32; 2], AtlasError> {
        let room = if self.coverage.fits(width, height) {
            self.evict_until_fits(width, height)
                .or_else(|()| self.grow_until_fits(width, height))
        } else {
            // Eviction cannot make a page larger: grow first, so the
            // cache survives (the growth keeps every glyph).
            self.grow_until_fits(width, height)
        };
        room.or_else(|full| self.room_unpinned((width, height), full))
    }

    /// Room once every pin is cleared; `full` when nothing was pinned.
    fn room_unpinned(
        &mut self,
        (width, height): (u32, u32),
        full: AtlasError,
    ) -> Result<[u32; 2], AtlasError> {
        if !self.unpin_all() {
            return Err(full);
        }
        self.evict_until_fits(width, height)
            .or_else(|()| self.grow_until_fits(width, height))
    }

    /// Room made by evicting the least recently used glyphs and
    /// repacking; `Err` when none can be evicted or the repack fails.
    fn evict_until_fits(&mut self, width: u32, height: u32) -> Result<[u32; 2], ()> {
        loop {
            let evicted = self.evict_least_recent(width * height);
            if evicted == 0 {
                return Err(());
            }
            self.evictions += evicted;
            self.repack().map_err(|_| ())?;
            if let Some(origin) = self.coverage.allocate(width, height) {
                return Ok(origin);
            }
        }
    }

    /// [`AtlasError::Full`] with what this frame has in the page.
    pub(crate) fn full(&self) -> AtlasError {
        let in_frame = || {
            self.entries
                .iter()
                .filter(|(_, entry)| entry.frame == self.frame)
        };
        let icons_this_frame = in_frame()
            .filter(|(key, _)| matches!(key, AtlasKey::Icon { .. }))
            .count();
        AtlasError::Full {
            page_size: self.coverage.size(),
            max_page_size: self.max_page,
            glyphs_this_frame: in_frame().count() - icons_this_frame,
            icons_this_frame,
        }
    }

    /// Drop evictable glyphs (not this frame's, not pinned) oldest first
    /// until at least `area` pixels of their bitmaps are gone; how many
    /// were dropped.
    fn evict_least_recent(&mut self, area: u32) -> u64 {
        let mut candidates: Vec<(u64, u32, AtlasKey)> = self
            .entries
            .iter()
            .filter(|(_, entry)| entry.frame != self.frame && entry.pins == 0)
            .map(|(key, entry)| {
                (
                    entry.last_used,
                    entry.glyph.rect[2] * entry.glyph.rect[3],
                    key.clone(),
                )
            })
            .collect();
        candidates.sort_unstable_by_key(|(last_used, _, _)| *last_used);
        let mut freed = 0;
        let mut evicted = 0;
        for (_, pixels, key) in candidates {
            if freed >= area && evicted > 0 {
                break;
            }
            self.evict(&key);
            freed += pixels;
            evicted += 1;
        }
        evicted
    }

    /// Clear the page and pack every surviving glyph again, tallest
    /// first, so the result does not depend on map order; the generation
    /// moves on. A survivor that no longer fits is evicted too, unless
    /// this frame needs it or it is pinned, which is [`AtlasError::Full`].
    pub(crate) fn repack(&mut self) -> Result<(), AtlasError> {
        self.coverage.clear();
        self.generation += 1;
        for key in self.survivors_tallest_first() {
            let entry = self.entries.get_mut(&key).expect("key came from the map");
            let (w, h) = (entry.coverage.width, entry.coverage.height);
            match self.coverage.allocate(w, h) {
                Some(origin) => {
                    self.coverage.blit(origin, w, h, &entry.coverage.data);
                    entry.glyph = glyph_at(origin, &entry.coverage);
                }
                None if entry.frame == self.frame || entry.pins > 0 => {
                    return Err(self.full());
                }
                None => {
                    self.evict(&key);
                    self.evictions += 1;
                }
            }
        }
        Ok(())
    }

    /// Keys of the inked glyphs, tallest first, then widest, then most
    /// recently used; a total order, so repacking is deterministic.
    fn survivors_tallest_first(&self) -> Vec<AtlasKey> {
        let mut survivors: Vec<(&AtlasKey, &Entry)> = self
            .entries
            .iter()
            .filter(|(_, entry)| !entry.coverage.is_empty())
            .collect();
        survivors.sort_unstable_by_key(|(_, entry)| {
            (entry.coverage.height, entry.coverage.width, entry.last_used)
        });
        survivors
            .into_iter()
            .rev()
            .map(|(key, _)| key.clone())
            .collect()
    }
}

#[cfg(test)]
#[path = "atlas_evict_tests.rs"]
mod tests;
