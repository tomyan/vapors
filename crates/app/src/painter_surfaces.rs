//! The painter's surface host: each mounted renderer by name, its
//! backing texture, and when it renders (docs/gpu-surface.md). A
//! surface renders on its first frame, after a resize or an
//! invalidation, and every frame while animating; other frames
//! composite its last render. A surface not painted in a frame is
//! inactive: its backing is released until it is painted again.

use std::collections::BTreeMap;

use vapors_elements::Painted;
use vapors_renderer::{ImagePipeline, Target};
use vapors_scene::{Item, Rect};

use super::ScenePainter;
use super::errors::{MountError, PaintError};
use crate::surface::{SurfaceRenderer, SurfaceSize};

#[path = "painter_surface_backing.rs"]
mod backing;
use backing::Mounted;

/// The mounted renderers, by surface name.
#[derive(Default)]
pub(super) struct Surfaces {
    mounted: BTreeMap<String, Mounted>,
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
        let mounted = self.surfaces.mounted.get_mut(name);
        let mounted = mounted.ok_or_else(|| MountError::NotMounted(name.to_owned()))?;
        mounted.invalidate();
        Ok(())
    }

    /// True when an active surface (one the last frame painted) is
    /// invalidated or animating: the app should draw another frame.
    #[must_use]
    pub fn wants_frame(&self) -> bool {
        self.surfaces.mounted.values().any(Mounted::wants_frame)
    }
}

impl Surfaces {
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
        for item in items {
            if let Item::Surface { surface, rect } = item {
                let name = painted.surface(*surface);
                let size = backing_size(*rect, painted.scale());
                let texture = self.prepared(target, name, size)?;
                pipeline.draw_rect(target, texture, *rect);
            }
        }
        Ok(())
    }

    fn prepared(
        &mut self,
        target: &Target<'_>,
        name: &str,
        size: SurfaceSize,
    ) -> Result<&wgpu::Texture, PaintError> {
        let mounted = self.mounted.get_mut(name);
        let mounted = mounted.ok_or_else(|| PaintError::Unmounted(name.to_owned()))?;
        mounted
            .prepare(target, size)
            .map_err(|refusal| refusal.named(name))
    }
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
