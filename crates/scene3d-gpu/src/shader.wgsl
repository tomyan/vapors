// The scene's one pipeline: positions through the mesh's world matrix
// and the camera's view-projection, filled with the mesh's colour times
// each vertex's shade (docs/plans/one-scene.md D4), its alpha kept. The
// multiply is on the sRGB-encoded channels, as the racer's baked shades
// were; not linear light.

struct Camera {
    view_projection: mat4x4<f32>,
};

struct Mesh {
    world: mat4x4<f32>,
    colour: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<uniform> mesh: Mesh;

struct Shaded {
    @builtin(position) position: vec4<f32>,
    @location(0) shade: f32,
};

@vertex
fn vs_main(@location(0) position: vec3<f32>, @location(1) shade: f32) -> Shaded {
    let clip = camera.view_projection * mesh.world * vec4<f32>(position, 1.0);
    return Shaded(clip, shade);
}

@fragment
fn fs_main(in: Shaded) -> @location(0) vec4<f32> {
    return vec4<f32>(mesh.colour.rgb * in.shade, mesh.colour.a);
}
