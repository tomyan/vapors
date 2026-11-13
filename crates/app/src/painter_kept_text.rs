//! The painter's text and icon batches kept on the GPU (IU4 PR 3,
//! `docs/plans/iu4.md` section 2): each batch's vertex bytes, in device
//! pixels, built on the first draw of its key with every glyph or icon
//! mask they name pinned in the atlas, written into the text pipeline's
//! kept buffer and drawn from there while they stay current. They are
//! built again when the atlas's generation moved on since they were
//! built (a repack moved glyphs, or every pin was cleared), or when an
//! item now names another layout or icon than they were built from: a
//! text id freed by a repaint can be issued to other text (risk 8), so
//! the resolved layouts are compared by identity, not the ids. A batch
//! with no ink keeps its pins and no range. Released with the frame's
//! other undrawn batches (`painter_kept.rs`), unpinning its glyphs.

use vapors_elements::{BatchKey, Painted};
use vapors_renderer::{KeptRange, Target};
use vapors_scene::Run;
use vapors_text::{AtlasError, AtlasPins, GlyphAtlas, TextPipeline};

use super::KeptGpu;
use resolved::Resolved;

#[path = "painter_kept_resolved.rs"]
mod resolved;
use crate::painter::text::run_bytes;
#[cfg(debug_assertions)]
use crate::painter::text::run_placed_bytes;

/// One text or icon batch's kept bytes.
pub(super) struct TextGpu {
    /// `None` for a batch with no ink.
    range: Option<KeptRange>,
    /// The pins on the glyphs the bytes name, and the atlas generation
    /// they were built at.
    pins: AtlasPins,
    /// What each item resolved to when the bytes were built.
    resolved: Resolved,
    len: u64,
    pub(super) drawn: u64,
    /// What was written, for the debug build's compare.
    #[cfg(debug_assertions)]
    bytes: Vec<u8>,
}

/// The text pipeline and the atlas its kept bytes name.
pub(crate) type TextParts<'a> = (&'a mut TextPipeline, &'a mut GlyphAtlas);

impl KeptGpu {
    /// The kept range of text or icon `run`, batch `at` of the frame,
    /// keyed `key` on `target`, and the pins it was built with: built
    /// and written first when none is current. `Ok(None)` when it has no
    /// ink or the write was refused (the refusal is on the target's
    /// encoder).
    ///
    /// # Errors
    ///
    /// [`AtlasError`] when its glyphs cannot be placed.
    pub(crate) fn text_range(
        &mut self,
        (text, atlas): TextParts<'_>,
        (target, painted): (&Target<'_>, &Painted),
        (run, key, at): (&Run, BatchKey, usize),
    ) -> Result<Option<(&KeptRange, &AtlasPins)>, AtlasError> {
        let generation = atlas.generation();
        match self.texts.get(&key) {
            Some(gpu) if gpu.current(run, painted, generation) => {
                gpu.check(atlas, painted, (run, at));
                self.kept += 1;
            }
            _ => self.write_text((text, atlas), (target, painted), (run, key))?,
        }
        let Some(gpu) = self.texts.get_mut(&key) else {
            return Ok(None);
        };
        gpu.drawn = self.frame;
        Ok(gpu.range.as_ref().map(|range| (range, &gpu.pins)))
    }

    /// Builds and writes `run`'s bytes under `key`, releasing those kept
    /// before; nothing kept when the write is refused.
    fn write_text(
        &mut self,
        (text, atlas): TextParts<'_>,
        (target, painted): (&Target<'_>, &Painted),
        (run, key): (&Run, BatchKey),
    ) -> Result<(), AtlasError> {
        if let Some(stale) = self.texts.remove(&key) {
            stale.release((text, atlas));
        }
        let (bytes, pins) = run_bytes(atlas, painted, run)?;
        let range = if bytes.is_empty() {
            None
        } else if let Some(range) = text.keep(target, &bytes) {
            Some(range)
        } else {
            atlas.unpin(pins);
            return Ok(());
        };
        let gpu = TextGpu::new((range, pins), (run, painted), (bytes, self.frame));
        self.texts.insert(key, gpu);
        self.written += 1;
        Ok(())
    }
}

impl TextGpu {
    /// The bytes built for `run` as `painted` resolves it, kept at
    /// `range` with `pins`, drawn in `frame`.
    fn new(
        (range, pins): (Option<KeptRange>, AtlasPins),
        (run, painted): (&Run, &Painted),
        (bytes, frame): (Vec<u8>, u64),
    ) -> Self {
        Self {
            range,
            pins,
            resolved: Resolved::of(run, painted),
            len: bytes.len() as u64,
            drawn: frame,
            #[cfg(debug_assertions)]
            bytes,
        }
    }

    /// True when these bytes are `run`'s as `painted` resolves it, with
    /// the atlas at `generation`.
    fn current(&self, run: &Run, painted: &Painted, generation: u64) -> bool {
        self.pins.generation() == generation && self.resolved.matches(run, painted)
    }

    /// The bytes held in the kept buffer.
    pub(super) fn len(&self) -> u64 {
        self.len
    }

    /// Gives the range back and unpins the glyphs.
    pub(super) fn release(self, (text, atlas): TextParts<'_>) {
        if let Some(range) = self.range {
            text.release(range);
        }
        atlas.unpin(self.pins);
    }

    /// In a debug build, panics when `run`'s bytes built now from where
    /// the atlas holds its glyphs differ from those kept, or a glyph is
    /// gone, naming batch `at`.
    #[cfg_attr(not(debug_assertions), allow(clippy::unused_self))]
    fn check(&self, atlas: &GlyphAtlas, painted: &Painted, (run, at): (&Run, usize)) {
        #[cfg(debug_assertions)]
        {
            let fresh = run_placed_bytes(atlas, painted, run).unwrap_or_else(|| {
                panic!(
                    "batch {at} ({run:?}) drew kept text bytes that name a glyph the atlas no \
                     longer holds: a pinned glyph was evicted (atlas_pins.rs), or bytes built \
                     before the atlas's generation moved on were kept (painter_kept_text.rs); \
                     reproduce with `cargo xtask test`"
                )
            });
            crate::painter::kept_check::assert_kept(&self.bytes, &fresh, (run, at));
        }
        #[cfg(not(debug_assertions))]
        let _ = (atlas, painted, run, at); // only the debug build's compare uses them
    }
}
