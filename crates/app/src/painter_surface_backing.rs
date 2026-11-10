//! One mounted surface: its renderer, the backing texture it renders
//! into, and whether the next paint must render it.

use vapors_renderer::Target;

use super::super::errors::Refusal;
use crate::surface::{SURFACE_FORMAT, SurfaceFrame, SurfaceRenderer, SurfaceSize};

/// A renderer and its state on the painter.
pub(super) struct Mounted {
    renderer: Box<dyn SurfaceRenderer>,
    /// Whether its `mount` has been called.
    started: bool,
    /// The texture it renders into; `None` while inactive.
    backing: Option<Backing>,
    /// Invalidated since its last render.
    stale: bool,
    /// Painted in the current frame.
    painted: bool,
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
            backing: None,
            stale: false,
            painted: false,
        }
    }

    pub(super) fn into_renderer(self) -> Box<dyn SurfaceRenderer> {
        self.renderer
    }

    pub(super) fn invalidate(&mut self) {
        self.stale = true;
    }

    pub(super) fn wants_frame(&self) -> bool {
        self.backing.is_some() && (self.stale || self.renderer.animating())
    }

    pub(super) fn begin_frame(&mut self) {
        self.painted = false;
    }

    pub(super) fn end_frame(&mut self) {
        if !self.painted {
            self.backing = None;
        }
    }

    /// The backing at `size`, rendered if it needs to be.
    pub(super) fn prepare(
        &mut self,
        target: &Target<'_>,
        size: SurfaceSize,
    ) -> Result<&wgpu::Texture, Refusal> {
        if std::mem::replace(&mut self.painted, true) {
            return Err(Refusal::DuplicateName);
        }
        fits(target.device, size)?;
        self.start(target.device)?;
        self.allocate(target.device, size)?;
        if self.stale || self.renderer.animating() {
            self.render(target)?;
        }
        Ok(&self.backing.as_ref().expect("allocated above").texture)
    }

    fn start(&mut self, device: &wgpu::Device) -> Result<(), Refusal> {
        if !self.started {
            self.renderer.mount(device).map_err(Refusal::Renderer)?;
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
        self.renderer
            .resize(device, size)
            .map_err(Refusal::Renderer)?;
        self.backing = Some(backing);
        self.stale = true;
        Ok(())
    }

    fn render(&mut self, target: &Target<'_>) -> Result<(), Refusal> {
        let backing = self.backing.as_ref().expect("allocated before rendering");
        let mut encoder = target
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("gpu-surface"),
            });
        let frame = SurfaceFrame {
            device: target.device,
            queue: target.queue,
            encoder: &mut encoder,
            view: &backing.view,
            size: backing.size,
        };
        self.renderer.render(frame).map_err(Refusal::Renderer)?;
        target.queue.submit([encoder.finish()]);
        self.stale = false;
        Ok(())
    }
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
        let texture = device.create_texture(&wgpu::TextureDescriptor {
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            texture,
            view,
            size,
        }
    }
}
