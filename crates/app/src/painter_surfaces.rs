//! The painter's surface host: each mounted renderer by name, its
//! backing texture, and when it renders (docs/gpu-surface.md). A
//! surface renders on its first frame, after a resize or an
//! invalidation, and every frame while animating (`set_animating`,
//! which `SurfaceHandle::request_animation` drives); other frames
//! composite its last render. A surface not painted in a frame, or
//! clipped out of view, is inactive: its backing is released until it
//! is painted again. A clipped-out surface is still refused as one in
//! view would be.

use std::collections::BTreeMap;
use std::task::Waker;

use vapors_elements::Painted;
use vapors_platform::Instant;
use vapors_renderer::{ImagePipeline, Target};
use vapors_scene::{Item, Rect, Run};

use super::errors::{MountError, PaintError};
use super::{ScenePainter, SurfaceState};
use crate::surface::{SurfaceRenderer, SurfaceSize};

#[path = "painter_surface_backing.rs"]
mod backing;
use backing::Mounted;
// For the root `gpu_` test module (the tier split needs a gpu_ id).
#[cfg(test)]
pub(crate) use backing::wants_frame_around_a_pending_scope;
#[path = "painter_surface_scope.rs"]
mod scope;
#[cfg(test)]
pub(crate) use scope::DEADLINE as SCOPE_DEADLINE;
use scope::Turn;
#[path = "painter_surface_texture.rs"]
mod texture;
#[path = "painter_surface_wake.rs"]
mod wake;

/// The mounted renderers, by surface name, and the waker a late GPU
/// error scope result wakes (`ScenePainter::wake_with`).
#[derive(Default)]
pub(super) struct Surfaces {
    mounted: BTreeMap<String, Mounted>,
    waker: Option<Waker>,
}

impl ScenePainter {
    /// Mounts `renderer` for the `gpu-surface` named `name`; its `mount`
    /// is called on the first frame that paints the surface.
    ///
    /// # Errors
    ///
    /// [`MountError::AlreadyMounted`] when a renderer has that name.
    pub fn mount(
        &mut self,
        name: &str,
        renderer: Box<dyn SurfaceRenderer>,
    ) -> Result<(), MountError> {
        let surfaces = &mut self.surfaces.mounted;
        if surfaces.contains_key(name) {
            return Err(MountError::AlreadyMounted(name.to_owned()));
        }
        surfaces.insert(name.to_owned(), Mounted::new(renderer));
        Ok(())
    }

    /// Unmounts the renderer named `name`, releasing its backing, and
    /// hands it back; dropping it is its teardown.
    ///
    /// # Errors
    ///
    /// [`MountError::NotMounted`] when no renderer has that name.
    pub fn unmount(&mut self, name: &str) -> Result<Box<dyn SurfaceRenderer>, MountError> {
        let mounted = self.surfaces.mounted.remove(name);
        mounted
            .map(Mounted::into_renderer)
            .ok_or_else(|| MountError::NotMounted(name.to_owned()))
    }

    /// Marks the surface named `name` for a render on the next frame
    /// that paints it. The caller asks for that frame.
    ///
    /// # Errors
    ///
    /// [`MountError::NotMounted`] when no renderer has that name.
    pub fn invalidate(&mut self, name: &str) -> Result<(), MountError> {
        self.surfaces.named(name)?.invalidate();
        Ok(())
    }

    /// Renders the surface named `name` on every frame that paints it
    /// while `animating`; a surface that is not painted renders nothing
    /// and asks for no frame. The caller asks for the next frame.
    ///
    /// # Errors
    ///
    /// [`MountError::NotMounted`] when no renderer has that name.
    pub fn set_animating(&mut self, name: &str, animating: bool) -> Result<(), MountError> {
        self.surfaces.named(name)?.set_animating(animating);
        Ok(())
    }
}

impl Surfaces {
    /// Each surface's state, by name.
    pub(super) fn states(&self) -> Vec<SurfaceState> {
        let named = self.mounted.iter();
        named.map(|(name, mounted)| mounted.state(name)).collect()
    }

    /// Marks every surface unpainted, before a frame.
    pub(super) fn begin_frame(&mut self) {
        self.mounted.values_mut().for_each(Mounted::begin_frame);
    }

    /// Releases the backing of each surface the frame did not paint.
    pub(super) fn end_frame(&mut self) {
        self.mounted.values_mut().for_each(Mounted::end_frame);
    }

    /// Renders, as needed, and composites each surface item of `items`.
    pub(super) fn draw(
        &mut self,
        pipeline: &ImagePipeline,
        target: &Target<'_>,
        painted: &Painted,
        items: &[Item],
    ) -> Result<(), PaintError> {
        let waker = self.waker.clone();
        for (name, size, rect) in surfaces(painted, items) {
            let mounted = self.mounted(name)?;
            let texture = mounted.prepare(target, size, turn(waker.as_ref()));
            pipeline.draw_rect(target, texture.map_err(|r| r.named(name))?, rect);
        }
        Ok(())
    }

    /// Refuses each surface of `run`, which is clipped out of view, as
    /// it would be refused in view (no renderer, a shared name, too
    /// large), without rendering it: it is inactive this frame.
    pub(super) fn check_clipped(
        &mut self,
        device: &wgpu::Device,
        painted: &Painted,
        run: &Run<'_>,
    ) -> Result<(), PaintError> {
        let Run::Surfaces(items) = run else {
            return Ok(());
        };
        let waker = self.waker.clone();
        for (name, size, _) in surfaces(painted, items) {
            let turn = turn(waker.as_ref());
            let claimed = self.mounted(name)?.claim(device, size, turn);
            claimed.map_err(|r| r.named(name))?;
        }
        Ok(())
    }

    fn named(&mut self, name: &str) -> Result<&mut Mounted, MountError> {
        let mounted = self.mounted.get_mut(name);
        mounted.ok_or_else(|| MountError::NotMounted(name.to_owned()))
    }

    fn mounted(&mut self, name: &str) -> Result<&mut Mounted, PaintError> {
        let mounted = self.mounted.get_mut(name);
        mounted.ok_or_else(|| PaintError::Unmounted(name.to_owned()))
    }
}

/// A turn now with `waker`, or one that wakes nothing.
fn turn(waker: Option<&Waker>) -> Turn<'_> {
    Turn {
        waker: waker.unwrap_or(Waker::noop()),
        now: Instant::now(),
    }
}

/// Each surface item of `items`: its name, backing size and rect.
fn surfaces<'a>(
    painted: &'a Painted,
    items: &'a [Item],
) -> impl Iterator<Item = (&'a str, SurfaceSize, Rect)> + 'a {
    items.iter().filter_map(|item| match item {
        Item::Surface { surface, rect } => Some((
            painted.surface(*surface),
            backing_size(*rect, painted.scale()),
            *rect,
        )),
        _ => None,
    })
}

/// The device-pixel size of a surface's snapped `rect` at `scale`.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // a painted rect is non-empty
fn backing_size(rect: Rect, scale: f32) -> SurfaceSize {
    SurfaceSize {
        width: rect.width().round() as u32,
        height: rect.height().round() as u32,
        scale,
    }
}
