//! The surface items of a run: each one's name, backing size and rect;
//! and each surface's backing made at its laid-out size at start.

use std::collections::BTreeSet;

use vapors_elements::Painted;
use vapors_renderer::ImagePipeline;
use vapors_scene::{Item, Rect};

use super::super::errors::PaintError;
use super::{Surfaces, turn, unmounted_refused};
use crate::surface::SurfaceSize;

impl Surfaces {
    /// At `App::start`, after the document's first update: each surface
    /// item of `painted` mounted, its backing made at its laid-out size
    /// and cleared, so a first frame at the start's size allocates
    /// nothing for it (at another size it remakes the backing). Every
    /// surface item is sized, clipped out of view or not: a clipped one's
    /// backing is dropped again by frame 0, a cost paid at start, not in
    /// a frame. Two items of one name are refused, as a frame refuses
    /// them.
    pub(in super::super) fn start_at(
        &mut self,
        gpu: (&wgpu::Device, &wgpu::Queue),
        pipeline: &ImagePipeline,
        painted: &Painted,
    ) -> Result<(), PaintError> {
        let mut seen = BTreeSet::new();
        for (item, _) in painted.scene().items() {
            let Item::Surface { surface, rect } = item else {
                continue;
            };
            let (name, size) = (
                painted.surface(surface),
                backing_size(rect, painted.scale()),
            );
            if !seen.insert(name) {
                return Err(PaintError::DuplicateName(name.to_owned()));
            }
            let mounted = unmounted_refused(&mut self.mounted, name)?;
            let started = mounted.start_at(gpu, pipeline, size, turn(&self.waker));
            started.map_err(|refusal| refusal.named(name))?;
        }
        Ok(())
    }
}

/// Each surface item of `items`: its name, backing size and rect.
pub(super) fn surfaces<'a>(
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
