//! One mounted surface: its renderer, the backing texture it renders
//! into, and whether the next paint must render it.

use vapors_renderer::Target;

use super::super::errors::Refusal;
use super::scope::Scopes;
use crate::surface::{SURFACE_FORMAT, SurfaceError, SurfaceFrame, SurfaceRenderer, SurfaceSize};

/// A renderer and its state on the painter.
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
    /// What the current frame did with it.
    seen: Seen,
    /// The GPU errors of its calls not yet seen.
    scopes: Scopes,
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

struct Backing {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: SurfaceSize,
}

impl Mounted {
    pub(super) fn new(renderer: Box<dyn SurfaceRenderer>) -> Self {
        Self {
            renderer,
            started: false,
            fresh: true,
            backing: None,
            stale: false,
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

    pub(super) fn wants_frame(&self) -> bool {
        self.fresh
            || (self.backing.is_some()
                && (self.stale || self.renderer.animating() || self.scopes.waiting()))
    }

    pub(super) fn begin_frame(&mut self) {
        self.seen = Seen::Unseen;
    }

    pub(super) fn end_frame(&mut self) {
        self.fresh = false;
        if self.seen != Seen::Painted {
            self.backing = None;
        }
    }

    /// The backing at `size`, rendered if it needs to be.
    pub(super) fn prepare(
        &mut self,
        target: &Target<'_>,
        size: SurfaceSize,
    ) -> Result<&wgpu::Texture, Refusal> {
        self.claim(target.device, size)?;
        self.seen = Seen::Painted;
        self.start(target.device)?;
        self.allocate(target.device, size)?;
        if self.stale || self.renderer.animating() {
            self.render(target)?;
        }
        Ok(&self.backing.as_ref().expect("allocated above").texture)
    }

    /// Claims the surface's name for the frame at `size`: refused when
    /// another surface claimed it, or when `size` is over the device's
    /// limit.
    pub(super) fn claim(
        &mut self,
        device: &wgpu::Device,
        size: SurfaceSize,
    ) -> Result<(), Refusal> {
        if std::mem::replace(&mut self.seen, Seen::Claimed) != Seen::Unseen {
            return Err(Refusal::DuplicateName);
        }
        self.scopes.check()?;
        fits(device, size)
    }

    fn start(&mut self, device: &wgpu::Device) -> Result<(), Refusal> {
        if !self.started {
            let renderer = &mut self.renderer;
            self.scopes.run(device, || renderer.mount(device))?;
            self.started = true;
        }
        Ok(())
    }

    /// A new backing when there is none at `size`; it needs a render.
    fn allocate(&mut self, device: &wgpu::Device, size: SurfaceSize) -> Result<(), Refusal> {
        if self.backing.as_ref().is_some_and(|held| held.size == size) {
            return Ok(());
        }
        let backing = Backing::new(device, size);
        let renderer = &mut self.renderer;
        self.scopes.run(device, || renderer.resize(device, size))?;
        self.backing = Some(backing);
        self.stale = true;
        Ok(())
    }

    /// Renders into the backing and submits it.
    fn render(&mut self, target: &Target<'_>) -> Result<(), Refusal> {
        let backing = self.backing.as_ref().expect("allocated before rendering");
        let renderer = &mut self.renderer;
        self.scopes
            .run(target.device, || render_into(renderer, target, backing))?;
        self.stale = false;
        Ok(())
    }
}

const ENCODER: wgpu::CommandEncoderDescriptor<'static> = wgpu::CommandEncoderDescriptor {
    label: Some("gpu-surface"),
};

/// `renderer`'s render into `backing`, submitted on `target`'s queue.
fn render_into(
    renderer: &mut Box<dyn SurfaceRenderer>,
    target: &Target<'_>,
    backing: &Backing,
) -> Result<(), SurfaceError> {
    let mut encoder = target.device.create_command_encoder(&ENCODER);
    renderer.render(SurfaceFrame {
        device: target.device,
        queue: target.queue,
        encoder: &mut encoder,
        view: &backing.view,
        size: backing.size,
    })?;
    target.queue.submit([encoder.finish()]);
    Ok(())
}

/// Refuses a backing larger than the device's largest 2D texture.
fn fits(device: &wgpu::Device, size: SurfaceSize) -> Result<(), Refusal> {
    let limit = device.limits().max_texture_dimension_2d;
    if size.width > limit || size.height > limit {
        return Err(Refusal::TooLarge { size, limit });
    }
    Ok(())
}

impl Backing {
    fn new(device: &wgpu::Device, size: SurfaceSize) -> Self {
        let texture = device.create_texture(&descriptor(size));
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            texture,
            view,
            size,
        }
    }
}

/// A backing is rendered into, sampled, and copyable both ways.
const USAGE: wgpu::TextureUsages = wgpu::TextureUsages::RENDER_ATTACHMENT
    .union(wgpu::TextureUsages::TEXTURE_BINDING)
    .union(wgpu::TextureUsages::COPY_SRC)
    .union(wgpu::TextureUsages::COPY_DST);

/// A [`SURFACE_FORMAT`] backing of `size`.
fn descriptor(size: SurfaceSize) -> wgpu::TextureDescriptor<'static> {
    wgpu::TextureDescriptor {
        label: Some("gpu-surface backing"),
        size: wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: SURFACE_FORMAT,
        usage: USAGE,
        view_formats: &[],
    }
}

#[cfg(test)]
#[path = "painter_surface_backing_tests.rs"]
mod tests;
// Re-exported under cfg(test) so the test can sit in a root `gpu_` module: the
// tier split matches `^gpu_` on the whole nextest id.
#[cfg(test)]
pub(crate) use tests::wants_frame_around_a_pending_scope;
