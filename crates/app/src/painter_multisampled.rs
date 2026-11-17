//! The painter's 4x colour textures (`docs/plans/flat-scene-f7.md`, plan
//! item 3): at 4 samples its passes draw into one of these and resolve
//! into the target's view. One for each viewport size it draws at (a
//! view of a smaller mip level is that level's size), made on the first
//! draw at that size. Keyed by size, not by view: an app makes a new
//! view of its surface's texture every frame, and a texture keyed by
//! view was made again every frame (41.5 MB a frame at 1080 x 2400, the
//! Pixel 9 Pro's crash, 2026-11-17).
//!
//! Kept while used: a texture goes once it has gone unused for as many
//! painter frames as the painter holds textures. Each `draw` is a
//! painter frame, so a painter drawing a fixed set of targets each app
//! frame, two sizes among them, draws at every size it holds within that
//! many of its frames and keeps them all; a size given up for good (a
//! resize) goes a frame or two later. Dropping each texture at the first
//! frame that did not draw at it made two textures every app frame for
//! a painter drawing two sizes (#480's review, M2).
//!
//! Two views of one size share the texture, so each records the view its
//! frame was cleared for, and a pass loading it for another (a tooltip
//! on A after B's draw) is refused: its resolve would put the other
//! frame here. One encoder records all of it: the pipelines refuse a
//! draw in a second encoder while the first is open (`encoder_turn.rs`),
//! so a tooltip cannot be recorded into an earlier encoder after another
//! view's draw. At 1 sample there are none.

use vapors_renderer::{PassFormat, Samples, Target};

pub(super) struct Multisampled {
    pass: PassFormat,
    held: Vec<Held>,
    /// Counts the frames: a texture drawn at in this one has it as
    /// `drawn`.
    frame: u64,
    /// Textures made since the last [`Multisampled::take_made`].
    made: u64,
}

/// What a pass does with the 4x texture first.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Use {
    /// Clears it for the target's frame (`draw`).
    Clear,
    /// Loads what the frame drew into it (`draw_tooltip`).
    Load,
}

/// A 4x texture: its size, its view, the target view its frame was
/// cleared for, and the frame that last drew at its size.
struct Held {
    size: (u32, u32),
    view: wgpu::TextureView,
    cleared_for: Option<wgpu::TextureView>,
    drawn: u64,
}

impl Multisampled {
    pub(super) fn new(pass: PassFormat) -> Self {
        Self {
            pass,
            held: Vec::new(),
            frame: 0,
            made: 0,
        }
    }

    /// The painter's sample count.
    pub(super) fn samples(&self) -> Samples {
        self.pass.samples
    }

    /// Starts a frame: nothing drawn yet.
    pub(super) fn begin_frame(&mut self) {
        self.frame += 1;
    }

    /// The 4x view for `target`'s viewport size (made first when it has
    /// none), drawn at this frame, for a pass that `uses` it; `None` at
    /// 1 sample.
    ///
    /// # Panics
    ///
    /// When `target` already names a multisampled view: the painter owns
    /// its own. When a pass would load a texture whose frame was cleared
    /// for another view: its resolve would put that view's frame here.
    pub(super) fn view_for(&mut self, target: &Target<'_>, uses: Use) -> Option<wgpu::TextureView> {
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
        let found = self.held.iter().position(|held| held.size == size);
        let at = found.unwrap_or_else(|| self.made_at(target, size));
        let held = &mut self.held[at];
        held.drawn = self.frame;
        match uses {
            Use::Clear => held.cleared_for = Some(target.view.clone()),
            Use::Load => assert_cleared_for(held, target),
        }
        Some(held.view.clone())
    }

    /// A new 4x texture at `size`; its place.
    fn made_at(&mut self, target: &Target<'_>, size: (u32, u32)) -> usize {
        self.made += 1;
        self.held.push(Held {
            size,
            view: made(target.device, self.pass, size),
            cleared_for: None,
            drawn: self.frame,
        });
        self.held.len() - 1
    }

    /// Drops the textures unused for as many frames as are held
    /// (the module's rule).
    pub(super) fn release_undrawn(&mut self) {
        let (frame, kept) = (self.frame, self.held.len() as u64);
        self.held.retain(|held| frame - held.drawn < kept);
    }

    /// The 4x textures made since the last take; the count restarts.
    pub(super) fn take_made(&mut self) -> u64 {
        std::mem::take(&mut self.made)
    }

    /// How many 4x textures are held.
    #[cfg(test)]
    pub(super) fn held(&self) -> usize {
        self.held.len()
    }

    /// The 4x view held at `size`, for the tests that it is kept.
    #[cfg(test)]
    pub(super) fn held_at(&self, size: (u32, u32)) -> Option<wgpu::TextureView> {
        let held = self.held.iter().find(|held| held.size == size);
        held.map(|held| held.view.clone())
    }
}

/// Refuses a load of `held` for `target` when its frame was cleared for
/// another view.
///
/// # Panics
///
/// As said, naming the fix.
fn assert_cleared_for(held: &Held, target: &Target<'_>) {
    let view = target.view;
    assert!(
        held.cleared_for.as_ref() == Some(view),
        "a 4-sample pass loads the {}x{} 4x texture for target view {view:?}, but its frame was \
         cleared for {:?}: its resolve would put that frame on this one; draw this target first, \
         and pass the same TextureView value (or a clone of it) that draw was given, not another \
         view of its texture (painter_multisampled.rs); reproduce with `cargo xtask test`",
        held.size.0,
        held.size.1,
        held.cleared_for
    );
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
