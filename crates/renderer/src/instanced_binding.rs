//! An instanced pipeline's shader source and its instance attributes
//! (`instanced.rs`); its bind group is `table_binding.rs`'s.

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
