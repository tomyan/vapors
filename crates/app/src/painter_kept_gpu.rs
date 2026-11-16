//! Each kept batch's bytes by its key, and which frame last drew it
//! (`painter_kept.rs`; text and icon batches in `painter_kept_text.rs`).

use std::collections::HashMap;

use vapors_elements::BatchKey;
use vapors_renderer::{KeptRange, Target};
use vapors_scene::Run;
use vapors_text::{GlyphAtlas, TextPipeline};

use super::super::errors::PaintError;
use super::{Kind, Pipelines, RunBytes, run_bytes};
use crate::shape_store::ShapeStore;

#[path = "painter_kept_text.rs"]
mod text;
use text::TextGpu;

/// The kept bytes of each batch drawn, by key.
#[derive(Default)]
pub(crate) struct KeptGpu {
    entries: HashMap<BatchKey, Gpu>,
    /// Text and icon batches' bytes, by key.
    texts: HashMap<BatchKey, TextGpu>,
    /// Counts the frames: an entry drawn in this one has it as `drawn`.
    frame: u64,
    /// Batches whose bytes were built and written this frame.
    written: u64,
    /// Batches drawn from bytes kept from an earlier frame.
    kept: u64,
}

/// A drawn batch's kept range, and a mesh run's items' entries by the
/// store's index.
pub(crate) struct Kept<'a> {
    pub(crate) range: &'a KeptRange,
    pub(crate) meshes: &'a [usize],
}

/// One batch's kept bytes.
struct Gpu {
    range: KeptRange,
    kind: Kind,
    /// How many bytes `range` holds.
    len: u64,
    /// A mesh run's items' entries, by the store's index, beside their
    /// instances in `range`: built once with the bytes.
    meshes: Vec<usize>,
    drawn: u64,
    /// What was written, for the debug build's compare.
    #[cfg(debug_assertions)]
    bytes: Vec<u8>,
}

impl KeptGpu {
    /// Starts a frame: nothing drawn or written yet.
    pub(crate) fn begin_frame(&mut self) {
        self.frame += 1;
        (self.written, self.kept) = (0, 0);
    }

    /// The frame's batches written, and drawn from kept bytes.
    pub(crate) fn drawn(&self) -> (u64, u64) {
        (self.written, self.kept)
    }

    /// The kept bytes of `run`, batch `at` of the frame, keyed `key` on
    /// `target`, written first when none are kept; `None` when the
    /// write is refused (on the target's encoder).
    ///
    /// # Errors
    /// A run whose bytes cannot be built ([`run_bytes`]).
    pub(super) fn range(
        &mut self,
        mut pipelines: Pipelines<'_>,
        target: &Target<'_>,
        (run, rows, key, at): (&Run, &[u32], BatchKey, usize),
    ) -> Result<Option<Kept<'_>>, PaintError> {
        match self.entries.get(&key) {
            Some(gpu) => {
                gpu.check((run, rows), at, pipelines.shapes);
                self.kept += 1;
            }
            _ => {
                if self
                    .write(&mut pipelines, target, (run, rows, key))?
                    .is_none()
                {
                    return Ok(None);
                }
            }
        }
        let gpu = self.entries.get_mut(&key).expect("kept or written above");
        gpu.drawn = self.frame;
        Ok(Some(Kept {
            range: &gpu.range,
            meshes: &gpu.meshes,
        }))
    }

    /// Builds and writes `run`'s bytes under `key`; `None` when the
    /// write is refused.
    fn write(
        &mut self,
        pipelines: &mut Pipelines<'_>,
        target: &Target<'_>,
        (run, rows, key): (&Run, &[u32], BatchKey),
    ) -> Result<Option<()>, PaintError> {
        let (kind, built) = (Kind::of(run), run_bytes((run, rows), pipelines.shapes)?);
        let RunBytes { bytes, meshes } = built;
        let Some(range) = pipelines.keep(kind, target, &bytes) else {
            return Ok(None);
        };
        self.written += 1;
        let gpu = Gpu {
            range,
            kind,
            len: bytes.len() as u64,
            meshes,
            drawn: self.frame,
            #[cfg(debug_assertions)]
            bytes,
        };
        self.entries.insert(key, gpu);
        Ok(Some(()))
    }

    pub(super) fn release_undrawn(&mut self, pipelines: &mut Pipelines<'_>) {
        let frame = self.frame;
        let undrawn = self.entries.extract_if(|_, gpu| gpu.drawn != frame);
        undrawn.for_each(|(_, gpu)| pipelines.release(gpu.kind, gpu.range));
        let undrawn = self.texts.extract_if(|_, gpu| gpu.drawn != frame);
        let text = (&mut *pipelines.text, &mut *pipelines.atlas);
        undrawn.for_each(|(_, gpu)| gpu.release((&mut *text.0, &mut *text.1)));
    }

    /// Gives back every text and icon batch's range and pins: the
    /// painter is going, and its atlas with it.
    pub(super) fn release_texts(&mut self, text: &mut TextPipeline, atlas: &mut GlyphAtlas) {
        for (_, gpu) in self.texts.drain() {
            gpu.release((&mut *text, &mut *atlas));
        }
    }

    /// The bytes the kept batches hold.
    pub(super) fn held_bytes(&self) -> u64 {
        let texts: u64 = self.texts.values().map(TextGpu::len).sum();
        self.entries.values().map(|gpu| gpu.len).sum::<u64>() + texts
    }
}

impl Gpu {
    /// In a debug build, panics when `run`'s bytes built afresh differ
    /// from those kept, naming batch `at`.
    #[cfg_attr(not(debug_assertions), allow(clippy::unused_self))]
    fn check(&self, (run, rows): (&Run, &[u32]), at: usize, shapes: &ShapeStore) {
        #[cfg(debug_assertions)]
        {
            let fresh = run_bytes((run, rows), shapes)
                .expect("built when these bytes were kept; the store only grows");
            super::super::kept_check::assert_kept(&self.bytes, &fresh.bytes, (run, at));
        }
        #[cfg(not(debug_assertions))]
        let _ = (run, rows, at, shapes); // only the debug build's compare uses them
    }
}
