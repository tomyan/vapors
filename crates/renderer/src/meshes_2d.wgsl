// The page's stages for mesh items (the rest is meshes_shared.wgsl): the
// item's affine transform takes the mesh's units to device pixels on the
// target, the same at every fragment.

struct View {
    size: vec2<f32>,
    shift: vec2<f32>,
}

@group(0) @binding(0) var<uniform> view: View;

@vertex
fn vs_main(@builtin(vertex_index) corner: u32, t: Triangle, item: Item) -> Out {
    let l = item.linear;
    let local = corner_of(t, corner) + local_move(corner_normals(t, corner), corner_kind(t, corner), l, item.place.z);
    let pixel = vec2(l.x * local.x + l.z * local.y, l.y * local.x + l.w * local.y) + item.place.xy;
    let on_target = pixel + view.shift;
    let position = vec4(on_target.x / view.size.x * 2.0 - 1.0, 1.0 - on_target.y / view.size.y * 2.0, 0.0, 1.0);
    return vertex_out(t, item, local, position, inverse_transpose(l, item.place.z));
}

@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    return covered(in, in.inverse_transpose, in.linear);
}
