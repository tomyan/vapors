//! One mounted surface: its renderer, the backing texture it renders
//! into (`painter_surface_texture.rs`), and whether the next paint must
//! render it.

use vapors_renderer::{ImagePipeline, Target, TextureBinding, WorkCounter};

use super::super::SurfaceState;
use super::super::errors::Refusal;
use super::scope::{Scopes, Turn};
use super::texture::{Backing, fits, render_into};
use crate::surface::{SurfaceRenderer, SurfaceSize};

/// A renderer and its state on the painter.
// four independent flags: each is set and cleared at its own time
#[allow(clippy::struct_excessive_bools)]
pub(super) struct Mounted {
    renderer: Box<dyn SurfaceRenderer>,
    /// Whether its `mount` has been called.
    started: bool,
    /// Mounted, and no frame has ended since: it wants one.
    fresh: bool,
    /// The texture it renders into; `None` while inactive.
    backing: Option<Backing>,
    /// Invalidated since its last render.
    stale: bool,
    /// Rendered on every frame that paints it.
    animating: bool,
    /// What the current frame did with it.
    seen: Seen,
    /// The GPU errors of its calls not yet seen.
    pub(super) scopes: Scopes,
}

/// What a frame did with a mounted surface.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Seen {
    /// No surface item named it.
    Unseen,
    /// A surface item named it, clipped out of view.
    Claimed,
    /// A surface item named it and it was drawn.
    Painted,
}

impl Mounted {
    pub(super) fn new(renderer: Box<dyn SurfaceRenderer>) -> Self {
        Self {
            renderer,
            started: false,
            fresh: true,
            backing: None,
            stale: false,
            animating: false,
            seen: Seen::Unseen,
            scopes: Scopes::default(),
        }
    }

    pub(super) fn into_renderer(self) -> Box<dyn SurfaceRenderer> {
        self.renderer
    }

    pub(super) fn invalidate(&mut self) {
        self.stale = true;
    }

    pub(super) fn set_animating(&mut self, animating: bool) {
        self.animating = animating;
    }

    pub(super) fn wants_frame(&self) -> bool {
        self.fresh || (self.backing.is_some() && (self.stale || self.animating))
    }

    pub(super) fn state(&self, name: &str) -> SurfaceState {
        SurfaceState {
            name: name.to_owned(),
            fresh: self.fresh,
            stale: self.stale,
            animating: self.animating,
            backing: self.backing.is_some(),
            waiting: self.scopes.waiting().is_some(),
        }
    }

    pub(super) fn begin_frame(&mut self) {
        self.seen = Seen::Unseen;
    }

    /// Ends the frame at `now`, settling its scopes even if unpainted:
    /// one left out of frames still gives up at its deadline.
    pub(super) fn end_frame(&mut self, now: vapors_platform::Instant) {
        self.scopes.settle(now);
        self.fresh = false;
        if self.seen != Seen::Painted {
            self.backing = None;
        }
    }

    /// The backing at `size`, bound for `pipeline`, rendered if it
    /// needs to be.
    pub(super) fn prepare(
        &mut self,
        target: &Target<'_>,
        pipeline: &ImagePipeline,
        size: SurfaceSize,
        turn: Turn<'_>,
        work: &WorkCounter,
    ) -> Result<&TextureBinding, Refusal> {
        self.claim(target.device, size, turn)?;
        self.seen = Seen::Painted;
        self.start(target.device, turn)?;
        self.allocate(target.device, pipeline, size, turn)?;
        if self.stale || self.animating {
            self.render(target, turn, work)?;
        }
        Ok(&self.backing.as_ref().expect("allocated above").bound)
    }

    /// Claims the surface's name for the frame at `size`: refused when
    /// another surface claimed it, or when `size` is over the device's
    /// limit.
    pub(super) fn claim(
        &mut self,
        device: &wgpu::Device,
        size: SurfaceSize,
        turn: Turn<'_>,
    ) -> Result<(), Refusal> {
        if std::mem::replace(&mut self.seen, Seen::Claimed) != Seen::Unseen {
            return Err(Refusal::DuplicateName);
        }
        self.scopes.check(turn.now)?;
        fits(device, size)
    }

    /// Calls the renderer's `mount` unless it was called already.
    pub(super) fn start(&mut self, device: &wgpu::Device, turn: Turn<'_>) -> Result<(), Refusal> {
        if !self.started {
            let renderer = &mut self.renderer;
            self.scopes.run(device, turn, || renderer.mount(device))?;
            self.started = true;
        }
        Ok(())
    }

    /// At `App::start`: mounted, and a backing at `size` made, the
    /// renderer resized, and the backing cleared on `queue`, so the
    /// memory both live in is the GPU's before the first frame: on
    /// i915 the first submit using a new memory block zeroes all of it
    /// (one-scene F0).
    pub(super) fn start_at(
        &mut self,
        (device, queue): (&wgpu::Device, &wgpu::Queue),
        pipeline: &ImagePipeline,
        size: SurfaceSize,
        turn: Turn<'_>,
    ) -> Result<(), Refusal> {
        fits(device, size)?;
        self.start(device, turn)?;
        self.allocate(device, pipeline, size, turn)?;
        let backing = self.backing.as_ref().expect("allocated above");
        // RELAXED: no test pins this clear. Its pixels are the zeros wgpu
        // fills a new texture with, and wgpu offers no count of submits,
        // so only frame 0's time shows it. Measured in diagnostic builds
        // on a loaded box (1-minute load 20 to 60), not acceptance runs:
        // without it frame 0's surface submit took 13 to 20 ms, with it
        // and the timer made before it 0.2 to 0.4 ms (one-scene.md, F0).
        // Retired by the acceptance runs, or a GPU-time test of frame 0.
        backing.clear(device, queue);
        Ok(())
    }

    /// A new backing when there is none at `size`; it needs a render.
    fn allocate(
        &mut self,
        device: &wgpu::Device,
        pipeline: &ImagePipeline,
        size: SurfaceSize,
        turn: Turn<'_>,
    ) -> Result<(), Refusal> {
        if self.backing.as_ref().is_some_and(|held| held.size == size) {
            return Ok(());
        }
        let backing = Backing::new(device, pipeline, size);
        let renderer = &mut self.renderer;
        self.scopes
            .run(device, turn, || renderer.resize(device, size))?;
        self.backing = Some(backing);
        self.stale = true;
        Ok(())
    }

    /// Renders into the backing, counting through `work`, and submits it.
    fn render(
        &mut self,
        target: &Target<'_>,
        turn: Turn<'_>,
        work: &WorkCounter,
    ) -> Result<(), Refusal> {
        let backing = self.backing.as_ref().expect("allocated before rendering");
        let renderer = &mut self.renderer;
        self.scopes.run(target.device, turn, || {
            render_into(renderer, target, backing, work)
        })?;
        self.stale = false;
        Ok(())
    }
}

#[cfg(test)]
#[path = "painter_surface_backing_tests.rs"]
mod tests;
// Re-exported under cfg(test) so the test can sit in a root `gpu_` module: the
// tier split matches `^gpu_` on the whole nextest id.
#[cfg(test)]
pub(crate) use tests::wants_frame_around_a_pending_scope;
