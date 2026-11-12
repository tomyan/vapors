//! Each kept batch's bytes by its key, and which frame last drew it
//! (`painter_kept.rs`).

use std::collections::HashMap;

use vapors_elements::BatchKey;
use vapors_renderer::{KeptRange, Target};
use vapors_scene::{Run, Size};

use super::{Kind, Pipelines, run_bytes};

/// The kept bytes of each batch drawn, by key.
#[derive(Default)]
pub(crate) struct KeptGpu {
    entries: HashMap<BatchKey, Gpu>,
    /// Counts the frames: an entry drawn in this one has it as `drawn`.
    frame: u64,
    /// Batches whose bytes were built and written this frame.
    written: u64,
    /// Batches drawn from bytes kept from an earlier frame.
    kept: u64,
}

/// One batch's kept bytes.
struct Gpu {
    range: KeptRange,
    kind: Kind,
    /// The viewport image and surface quads are in clip space for;
    /// `None` for shadows and quads, whose bytes are in device pixels.
    viewport: Option<Size>,
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
    /// `target`, written first when none are kept for its viewport;
    /// `None` when refused.
    pub(super) fn range(
        &mut self,
        mut pipelines: Pipelines<'_>,
        target: &Target<'_>,
        (run, key, at): (&Run, BatchKey, usize),
    ) -> Option<&KeptRange> {
        let viewport = target.viewport;
        match self.entries.get(&key) {
            Some(gpu) if gpu.viewport.is_none_or(|kept| kept == viewport) => {
                #[cfg(debug_assertions)]
                super::super::kept_check::assert_kept(
                    &gpu.bytes,
                    &run_bytes(run, viewport),
                    (run, at),
                );
                #[cfg(not(debug_assertions))]
                let _ = at; // only the debug build's compare names the batch
                self.kept += 1;
            }
            _ => self.write(&mut pipelines, target, (run, key))?,
        }
        let gpu = self.entries.get_mut(&key).expect("kept or written above");
        gpu.drawn = self.frame;
        Some(&gpu.range)
    }

    /// Builds and writes `run`'s bytes under `key`, releasing those kept
    /// for another viewport; `None` when refused.
    fn write(
        &mut self,
        pipelines: &mut Pipelines<'_>,
        target: &Target<'_>,
        (run, key): (&Run, BatchKey),
    ) -> Option<()> {
        if let Some(stale) = self.entries.remove(&key) {
            pipelines.release(stale.kind, stale.range);
        }
        let (kind, bytes) = (Kind::of(run), run_bytes(run, target.viewport));
        let range = pipelines.keep(kind, target, &bytes)?;
        self.written += 1;
        let gpu = Gpu {
            range,
            kind,
            viewport: (kind == Kind::Images).then_some(target.viewport),
            drawn: self.frame,
            #[cfg(debug_assertions)]
            bytes,
        };
        self.entries.insert(key, gpu);
        Some(())
    }

    pub(super) fn release_undrawn(&mut self, pipelines: &mut Pipelines<'_>) {
        let frame = self.frame;
        let undrawn = self.entries.extract_if(|_, gpu| gpu.drawn != frame);
        undrawn.for_each(|(_, gpu)| pipelines.release(gpu.kind, gpu.range));
    }
}
