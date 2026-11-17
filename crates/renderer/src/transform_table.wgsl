// The transform table (transform_table.rs, docs/plans/flat-scene-f2.md):
// each instance names its frame's entry and its row's entry in its
// batch's range, which starts at view.rows_base; a position is moved by
// the row's transform, then the frame's. Shared by the instanced
// pipelines (instanced.rs).

// The target's size, and the first entry of the batch's row range.
struct View {
    size: vec2<f32>,
    rows_base: u32,
}

@group(0) @binding(0) var<uniform> view: View;

@group(0) @binding(1) var<storage, read> transforms: array<mat4x4<f32>>;

// `pixel`, in its frame's coordinates and its row's, on the target.
fn placed(pixel: vec2<f32>, frame: u32, row: u32) -> vec2<f32> {
    let m = transforms[frame] * transforms[view.rows_base + row];
    return (m * vec4(pixel, 0.0, 1.0)).xy;
}

// `on_target` (device pixels) in clip space.
fn clip_position(on_target: vec2<f32>) -> vec4<f32> {
    return vec4(on_target.x / view.size.x * 2.0 - 1.0, 1.0 - on_target.y / view.size.y * 2.0, 0.0, 1.0);
}
