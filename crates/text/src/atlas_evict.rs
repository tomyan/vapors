//! Making room in a full page: evict the least recently used glyphs that
//! the current frame has not asked for, repack the survivors in a fixed
//! order, and try again until the new glyph fits.

use crate::atlas::{Entry, GlyphAtlas, glyph_at};
use crate::atlas_types::{AtlasError, GlyphKey};

impl GlyphAtlas {
    /// Room for a `width` by `height` glyph, or [`AtlasError::Full`] when
    /// every glyph in the page belongs to this frame.
    pub(crate) fn make_room(&mut self, width: u32, height: u32) -> Result<[u32; 2], AtlasError> {
        loop {
            let evicted = self.evict_least_recent(width * height);
            if evicted == 0 {
                return Err(AtlasError::Full {
                    page_size: self.coverage.size(),
                    used_this_frame: self.entries.len(),
                });
            }
            self.evictions += evicted;
            self.repack()?;
            if let Some(origin) = self.coverage.allocate(width, height) {
                return Ok(origin);
            }
        }
    }

    /// Drop evictable glyphs oldest first until at least `area` pixels of
    /// their bitmaps are gone; how many were dropped.
    fn evict_least_recent(&mut self, area: u32) -> u64 {
        let mut candidates: Vec<(u64, u32, GlyphKey)> = self
            .entries
            .iter()
            .filter(|(_, entry)| entry.frame != self.frame)
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
            self.entries.remove(&key);
            freed += pixels;
            evicted += 1;
        }
        evicted
    }

    /// Clear the page and pack every surviving glyph again, tallest
    /// first, so the result does not depend on map order. A survivor
    /// that no longer fits is evicted too, unless this frame needs it,
    /// which is [`AtlasError::Full`].
    pub(crate) fn repack(&mut self) -> Result<(), AtlasError> {
        self.coverage.clear();
        for key in self.survivors_tallest_first() {
            let entry = self.entries.get_mut(&key).expect("key came from the map");
            let (w, h) = (entry.coverage.width, entry.coverage.height);
            match self.coverage.allocate(w, h) {
                Some(origin) => {
                    self.coverage.blit(origin, w, h, &entry.coverage.data);
                    entry.glyph = glyph_at(origin, &entry.coverage);
                }
                None if entry.frame == self.frame => {
                    return Err(AtlasError::Full {
                        page_size: self.coverage.size(),
                        used_this_frame: self.entries.len(),
                    });
                }
                None => {
                    self.entries.remove(&key);
                    self.evictions += 1;
                }
            }
        }
        Ok(())
    }

    /// Keys of the inked glyphs, tallest first, then widest, then most
    /// recently used; a total order, so repacking is deterministic.
    fn survivors_tallest_first(&self) -> Vec<GlyphKey> {
        let mut survivors: Vec<(&GlyphKey, &Entry)> = self
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
