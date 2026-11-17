//! The text pipeline's atlas texture, its view and the bind group made
//! over them, changed together: replacing the texture makes its view
//! and drops the kept bind group, so no draw samples a texture the
//! pipeline no longer uploads to.

use vapors_renderer::KeptBindGroup;

use crate::pipeline_gpu::coverage_texture;

/// The coverage texture glyphs are uploaded to and drawn from.
pub(crate) struct AtlasTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    /// Over the pipeline's own identity table, for immediate draws.
    own_group: KeptBindGroup<[u64; 3]>,
    /// Over a painter's transform and clip tables, for kept draws.
    table_group: KeptBindGroup<[u64; 3]>,
}

impl AtlasTexture {
    /// An empty texture of `size` x `size` px, no bind group yet.
    pub(crate) fn new(device: &wgpu::Device, size: u32) -> Self {
        let texture = coverage_texture(device, size);
        Self {
            view: texture.create_view(&wgpu::TextureViewDescriptor::default()),
            texture,
            own_group: KeptBindGroup::NONE,
            table_group: KeptBindGroup::NONE,
        }
    }

    /// Replaces the texture with an empty one of `size` x `size` px: the
    /// only way to change it, so the view and bind group follow.
    pub(crate) fn replace(&mut self, device: &wgpu::Device, size: u32) {
        *self = Self::new(device, size);
    }

    pub(crate) fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    /// The bind group over the uniform buffer, transform table and clip
    /// table of `generation`, for the pipeline's own tables (`own`) or a
    /// painter's: the kept one, or one `make` makes over the view.
    pub(crate) fn bind_group(
        &mut self,
        (own, generation): (bool, [u64; 3]),
        make: impl FnOnce(&wgpu::TextureView) -> wgpu::BindGroup,
    ) -> &wgpu::BindGroup {
        let Self {
            view,
            own_group,
            table_group,
            ..
        } = self;
        let kept = if own { own_group } else { table_group };
        kept.of(generation, || make(view))
    }

    /// How many bind groups are kept: 0, 1 or 2.
    pub(crate) fn bind_groups_kept(&self) -> u64 {
        self.own_group.kept() + self.table_group.kept()
    }
}
