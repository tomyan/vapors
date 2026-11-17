// A mesh drawn by an application material (vapors-scene3d `material.rs`):
// positions through the mesh's world matrix and the camera's
// view-projection, as `shader.wgsl`'s; the fragment's colour is the
// material's, given what `MaterialInput` holds. `camera` and `node` are
// `vapors_renderer::PROJECTIVE_WGSL`'s, then `MATERIAL_WGSL` and the
// material's own WGSL, which pipeline.rs puts before this.

struct MaterialVertex {
    @builtin(position) position: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) shade: f32,
};

@vertex
fn vs_material(@location(0) position: vec3<f32>, @location(1) shade: f32) -> MaterialVertex {
    let world = node.world * vec4<f32>(position, 1.0);
    return MaterialVertex(camera.view_projection * world, world.xyz, shade);
}

@fragment
fn fs_material(in: MaterialVertex) -> @location(0) vec4<f32> {
    return material(MaterialInput(in.world, in.position.xy, camera.size, node.colour, in.shade));
}
