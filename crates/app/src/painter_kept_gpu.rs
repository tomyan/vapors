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

/// The transform and clip entries a run's items name.
type Named = (u32, u32);

/// A batch's rows' indices, and how many rows its range holds.
type RowsIn<'a> = (&'a [u32], usize);

/// A run to draw, batch `at` of the frame, keyed `key`: what its items
/// name and its rows.
type Drawn<'a> = (&'a Run, (Named, &'a [u32], usize), BatchKey, usize);

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
    /// The frame's entry in the transform table and the clip's in the
    /// clip table its items name (0 for the runs that name none): bytes
    /// naming others are built again.
    frame: u32,
    clip: u32,
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
    /// `target`, its items in the transform table's entry `frame`;
    /// written first when none are kept, or the kept ones name another
    /// entry. `None` when the write is refused (on the target's encoder).
    ///
    /// # Errors
    /// A run whose bytes cannot be built ([`run_bytes`]).
    pub(super) fn range(
        &mut self,
        mut pipelines: Pipelines<'_>,
        target: &Target<'_>,
        (run, ((frame, clip), rows, range), key, at): Drawn<'_>,
    ) -> Result<Option<Kept<'_>>, PaintError> {
        if let Some(stale) = self
            .entries
            .extract_if(|k, gpu| *k == key && (gpu.frame, gpu.clip) != (frame, clip))
            .next()
        {
            pipelines.release(stale.1.kind, stale.1.range);
        }
        match self.entries.get(&key) {
            Some(gpu) => {
                #[cfg(debug_assertions)]
                super::rows_in_range(key, rows, range);
                gpu.check((run, (frame, clip), rows), at, pipelines.shapes);
                self.kept += 1;
            }
            _ => {
                if self
                    .write(
                        &mut pipelines,
                        target,
                        (run, (frame, clip), (rows, range), key),
                    )?
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
        (run, (frame, clip), (rows, range), key): (&Run, Named, RowsIn<'_>, BatchKey),
    ) -> Result<Option<()>, PaintError> {
        super::rows_in_range(key, rows, range);
        let (kind, built) = (
            Kind::of(run),
            run_bytes((run, (frame, clip), rows), pipelines.shapes)?,
        );
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
            frame,
            clip,
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

    /// Panics, naming the batch, when a kept batch names one of the clip
    /// entries `freed`: the CPU check behind the poison a freed entry is
    /// written with (flat-scene-f3.md item 11). Undrawn batches are
    /// released first, and a drawn one holds its entries, so none does.
    pub(super) fn assert_none_names_clip(&self, freed: &[u32]) {
        let named = self
            .entries
            .iter()
            .find(|(_, gpu)| freed.contains(&gpu.clip));
        if let Some((key, gpu)) = named {
            panic!(
                "kept batch {key:?} names clip entry {}, given back: its bytes would read the \
                 poison or another clip (painter_kept_gpu.rs, painter_clips.rs); reproduce with \
                 `cargo xtask test`",
                gpu.clip
            );
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
    fn check(
        &self,
        (run, frame, rows): (&Run, (u32, u32), &[u32]),
        at: usize,
        shapes: &ShapeStore,
    ) {
        #[cfg(debug_assertions)]
        {
            let fresh = run_bytes((run, frame, rows), shapes)
                .expect("built when these bytes were kept; the store only grows");
            super::super::kept_check::assert_kept(&self.bytes, &fresh.bytes, (run, at));
        }
        #[cfg(not(debug_assertions))]
        let _ = (run, frame, rows, at, shapes); // only the debug build's compare uses them
    }
}
