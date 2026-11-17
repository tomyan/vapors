// The page's stages for mesh items (the rest is meshes_shared.wgsl): the
// item's affine transform takes the mesh's units to device pixels in its
// frame, the same at every fragment, and the transform table
// (transform_table.wgsl) places them on the target.

@vertex
fn vs_main(
    @builtin(vertex_index) corner: u32,
    t: Triangle,
    item: Item,
    // the row's index in its batch's range, the frame's entry in the
    // transform table
    @location(15) place: vec2<u32>,
) -> Out {
    let l = item.linear;
    let local = corner_of(t, corner) + local_move(corner_normals(t, corner), corner_kind(t, corner), l, item.place.z);
    let pixel = vec2(l.x * local.x + l.z * local.y, l.y * local.x + l.w * local.y) + item.place.xy;
    let position = clip_position(placed(pixel, place.y, place.x));
    return vertex_out(t, item, local, position, inverse_transpose(l, item.place.z));
}

@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    return covered(in, in.inverse_transpose, in.linear);
}
