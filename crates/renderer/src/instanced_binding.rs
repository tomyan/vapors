//! An instanced pipeline's shader source, its bind group over the view
//! slots and the two tables, and its instance attributes (`instanced.rs`).

use crate::instanced_bytes::VIEW_SIZE;
use crate::uniform_slots::UniformSlots;
use crate::{ClipTable, TransformTable, WorkCounter};

/// An instanced pipeline's shader source: the transform table, the clip
/// table's prelude and shader (`clip.wgsl`), then the pipeline's own.
pub(crate) fn instanced_source(own: &str) -> String {
    format!(
        "{}{}{}{own}",
        include_str!("transform_table.wgsl"),
        crate::clip_prelude(),
        include_str!("clip.wgsl")
    )
}

pub(crate) fn group(
    device: &wgpu::Device,
    (label, layout, work): (&str, &wgpu::BindGroupLayout, &WorkCounter),
    (viewports, table, clips): (&UniformSlots<VIEW_SIZE>, &TransformTable, &ClipTable),
) -> wgpu::BindGroup {
    work.create_bind_group(
        device,
        &wgpu::BindGroupDescriptor {
            label: Some(label),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: viewports.binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: table.binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: clips.binding(),
                },
            ],
        },
    )
}

/// The instance attributes: `vec4s` four-float ones, then the row's
/// index, the frame's entry and the clip's entry, three `u32`s.
pub(crate) fn attributes(vec4s: u32) -> Vec<wgpu::VertexAttribute> {
    let floats = u64::from(vec4s) * 16;
    let index = |at: u32| wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Uint32,
        offset: floats + u64::from(at) * 4,
        shader_location: vec4s + at,
    };
    (0..vec4s)
        .map(|i| wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: u64::from(i) * 16,
            shader_location: i,
        })
        .chain([index(0), index(1), index(2)])
        .collect()
}
