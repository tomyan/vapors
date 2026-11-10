//! What the painter's last frame handed the GPU, by the kind of run that
//! issued it: counted on the CPU as each write and draw is issued, with
//! no readback and no wait.

use vapors_renderer::GpuWork;
use vapors_scene::Run;

use super::ScenePainter;

/// GPU work the painter's last frame ([`ScenePainter::draw`] or
/// [`ScenePainter::draw_over`]) issued, per kind of scene run, for
/// diagnostics and tests. Structural: bytes and calls, not time.
///
/// Reset when a frame starts; each run adds its work after it draws,
/// refused or not, so a refused frame holds the runs drawn before the
/// refusal and the part of the refused run issued before it. A refused
/// `draw_over` (a canvas background) draws nothing and leaves the frame
/// before it. Not counted: the clear (a render pass with no draw), and
/// what a `gpu-surface`'s own renderer issues on the queue or encoder
/// directly rather than through `SurfaceFrame::work`.
/// Before the first frame, all are 0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct FrameCounts {
    /// Shadow runs: one draw per run, its instances and the viewport.
    pub shadows: GpuWork,
    /// Quad runs: one draw per run, its instances and the viewport.
    pub quads: GpuWork,
    /// Text runs: one draw per text item, its vertices and blend
    /// uniform; the part of the atlas page its new glyphs changed.
    pub texts: GpuWork,
    /// Icon runs: one draw per run, its vertices and blend uniform; the
    /// part of the atlas page its new icon masks changed.
    pub icons: GpuWork,
    /// Image runs: one draw per image item and its quad's vertices; an
    /// image's pixels, every mip level, the first frame that draws it.
    pub images: GpuWork,
    /// `gpu-surface` runs: one composition draw per surface item and its
    /// quad's vertices, and what each surface's renderer issued through
    /// `SurfaceFrame::work` in the frames that render it.
    pub surfaces: GpuWork,
}

impl FrameCounts {
    /// The frame's work over every kind of run.
    #[must_use]
    pub fn total(&self) -> GpuWork {
        self.shadows + self.quads + self.texts + self.icons + self.images + self.surfaces
    }

    /// Bytes of glyph atlas uploaded: by text runs and icon runs.
    #[must_use]
    pub fn atlas_bytes(&self) -> u64 {
        self.texts.texture_bytes + self.icons.texture_bytes
    }

    fn of_kind(&mut self, run: &Run<'_>) -> &mut GpuWork {
        match run {
            Run::Shadows(_) => &mut self.shadows,
            Run::Quads(_) => &mut self.quads,
            Run::Texts(_) => &mut self.texts,
            Run::Icons(_) => &mut self.icons,
            Run::Images(_) => &mut self.images,
            Run::Surfaces(_) => &mut self.surfaces,
        }
    }
}

impl ScenePainter {
    /// What the last frame handed the GPU: see [`FrameCounts`].
    #[must_use]
    pub fn frame_counts(&self) -> FrameCounts {
        self.counts
    }

    /// Starts a frame's counts at 0.
    pub(super) fn begin_counts(&mut self) {
        self.counts = FrameCounts::default();
        let leftover = self.take_work();
        debug_assert_eq!(
            leftover,
            GpuWork::default(),
            "work issued outside a counted run; every run goes through draw_run"
        );
    }

    /// Adds the work issued since the last take to `run`'s kind.
    pub(super) fn count_run(&mut self, run: &Run<'_>) {
        let work = self.take_work();
        *self.counts.of_kind(run) += work;
    }

    /// The work every pipeline issued since the last take. One run draws
    /// at a time, so it is that run's.
    fn take_work(&self) -> GpuWork {
        self.shadows.take_work()
            + self.quads.take_work()
            + self.text.take_work()
            + self.images.take_work()
            + self.textures.take_work()
            + self.surfaces.take_work()
    }
}
