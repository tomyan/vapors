//! GPU tier: replacing the atlas texture remakes the bind group over
//! the new view.

use vapors_gpu_harness::gpu;
use vapors_renderer::{UniformSlots, WorkCounter};

use crate::atlas_texture::AtlasTexture;
use crate::pipeline::bind_group;
use crate::pipeline_gpu::bind_group_layout;

/// Bind groups `atlas` made for one draw against `params`.
fn made_for_a_draw(device: &wgpu::Device, atlas: &mut AtlasTexture, params: &UniformSlots) -> u64 {
    let work = WorkCounter::default();
    let layout = bind_group_layout(device);
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
    atlas.bind_group(params.generation(), |view| {
        bind_group(device, &work, &layout, view, &sampler, params)
    });
    work.take().bind_groups_made
}

#[test]
fn gpu_a_replaced_atlas_texture_gets_a_new_bind_group() {
    // Given: a 64 px texture drawn twice
    let gpu = gpu();
    let params = UniformSlots::new(&gpu.device, "text params");
    let mut atlas = AtlasTexture::new(&gpu.device, 64);
    let first = made_for_a_draw(&gpu.device, &mut atlas, &params);
    let again = made_for_a_draw(&gpu.device, &mut atlas, &params);

    // When
    atlas.replace(&gpu.device, 128);
    let replaced = made_for_a_draw(&gpu.device, &mut atlas, &params);

    // Then
    assert_eq!((first, again, replaced), (1, 0, 1));
    assert_eq!(atlas.texture().width(), 128);
    assert_eq!(atlas.bind_groups_kept(), 1);
}
