// The scene's stages for mesh items (the rest is meshes_shared.wgsl and
// projective.wgsl; docs/plans/decal.md section 2). The vertex stage
// moves each outline corner half a device pixel out through the screen
// Jacobian at that corner, in place of the page's constant affine. The
// fragment stage takes the map from shape units to device pixels at each
// fragment from the derivatives of the shape-unit position, which under
// a projection are its local inverse: their transpose is the map's
// inverse transpose, which the wedge distances divide by, as the closed
// forms take their rate across the screen.

@vertex
fn vs_main(@builtin(vertex_index) corner: u32, t: Triangle, item: Item) -> Out {
    let at = corner_of(t, corner);
    let j = screen_jacobian(item.linear, item.place.xy, at);
    let det = j.x * j.w - j.y * j.z;
    var moved = vec2(0.0);
    if (abs(det) > 1e-12) {
        moved = local_move(corner_normals(t, corner), corner_kind(t, corner), j, 1.0 / det);
    }
    let local = at + moved;
    let position = to_clip() * vec4(on_plane(item.linear, item.place.xy, local), 0.0, 1.0);
    return vertex_out(t, item, local, position, vec4(0.0));
}

@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    let across_x = dpdx(in.local);
    let across_y = dpdy(in.local);
    let t = vec4(across_x, across_y);
    let det = across_x.x * across_y.y - across_y.x * across_x.y;
    let safe = select(1e-20, det, abs(det) > 1e-20);
    let l = vec4(across_y.y, -across_x.y, -across_y.x, across_x.x) / safe;
    return covered(in, t, l);
}
