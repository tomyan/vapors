//! The painter's 4x colour textures (`docs/plans/flat-scene-f7.md`, plan
//! item 3): at 4 samples its passes draw into one of these and resolve
//! into the target's view. One for each target view it draws to, at the
//! viewport's size (a view of a smaller mip level is that level's size):
//! a texture holds one view's frame, so a pass loading it resolves that
//! view's frame and no other's. Made on the first draw to the view, kept
//! while each frame draws to it, and dropped after a frame that does
//! not, the rule kept bytes and table entries follow. Each `draw` is a
//! frame, so a painter drawing two targets an app frame remakes each
//! one's texture on each call, and a tooltip on a target after another
//! target's `draw` meets a fresh texture, which F7a1's uncleared-load
//! check refuses. At 1 sample there are none.

use vapors_renderer::{PassFormat, Samples, Target};

pub(super) struct Multisampled {
    pass: PassFormat,
    held: Vec<Held>,
    /// Counts the frames: a texture drawn to in this one has it as
    /// `drawn`.
    frame: u64,
}

/// A 4x texture: the target view it resolves into and its size, its
/// view, and the frame that last drew to it.
struct Held {
    target: wgpu::TextureView,
    size: (u32, u32),
    view: wgpu::TextureView,
    drawn: u64,
}

impl Multisampled {
    pub(super) fn new(pass: PassFormat) -> Self {
        Self {
            pass,
            held: Vec::new(),
            frame: 0,
        }
    }

    /// The painter's sample count.
    pub(super) fn samples(&self) -> Samples {
        self.pass.samples
    }

    /// Starts a frame: nothing drawn to yet.
    pub(super) fn begin_frame(&mut self) {
        self.frame += 1;
    }

    /// The 4x view of `target`'s view at its viewport's size (made first
    /// when it has none), drawn to this frame; `None` at 1 sample.
    ///
    /// # Panics
    ///
    /// When `target` already names a multisampled view: the painter owns
    /// its own, and a 1-sample painter's pipelines cannot draw into one.
    pub(super) fn view_for(&mut self, target: &Target<'_>) -> Option<wgpu::TextureView> {
        assert!(
            target.multisampled.is_none(),
            "a painter was given a target with a multisampled view: the painter makes its own at \
             4 samples and draws none at 1 (painter_multisampled.rs); pass the target's view \
             alone; reproduce with `cargo xtask test`"
        );
        if self.pass.samples == Samples::One {
            return None;
        }
        let size = (target.viewport.width, target.viewport.height);
        let found = self
            .held
            .iter()
            .position(|held| held.target == *target.view && held.size == size);
        let at = found.unwrap_or_else(|| self.made_for(target, size));
        self.held[at].drawn = self.frame;
        Some(self.held[at].view.clone())
    }

    /// A new 4x texture for `target`'s view at `size`; its place.
    fn made_for(&mut self, target: &Target<'_>, size: (u32, u32)) -> usize {
        self.held.push(Held {
            target: target.view.clone(),
            size,
            view: made(target.device, self.pass, size),
            drawn: self.frame,
        });
        self.held.len() - 1
    }

    /// Drops the textures this frame did not draw to.
    pub(super) fn release_undrawn(&mut self) {
        let frame = self.frame;
        self.held.retain(|held| held.drawn == frame);
    }

    /// How many 4x textures are held.
    #[cfg(test)]
    pub(super) fn held(&self) -> usize {
        self.held.len()
    }

    /// The 4x view held for `target`, for the tests that it is kept.
    #[cfg(test)]
    pub(super) fn held_for(&self, target: &wgpu::TextureView) -> Option<wgpu::TextureView> {
        let held = self.held.iter().find(|held| held.target == *target);
        held.map(|held| held.view.clone())
    }
}

/// `target` drawing through `multisampled` (none at 1 sample).
pub(super) fn attached<'a>(
    target: Target<'a>,
    multisampled: Option<&'a wgpu::TextureView>,
) -> Target<'a> {
    Target {
        multisampled,
        ..target
    }
}

/// A 4x colour texture of `pass`'s format, `width` by `height`, and its
/// view.
fn made(device: &wgpu::Device, pass: PassFormat, (width, height): (u32, u32)) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("painter multisampled"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: pass.samples.count(),
        dimension: wgpu::TextureDimension::D2,
        format: pass.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

#[cfg(test)]
#[path = "painter_multisampled_tests.rs"]
mod tests;
