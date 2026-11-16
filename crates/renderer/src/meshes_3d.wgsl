// The scene's stages for mesh items (the rest is meshes_shared.wgsl and
// projective.wgsl; docs/plans/decal.md section 2). The vertex stage
// moves each outline corner half a device pixel out through the screen
// Jacobian at that corner, in place of the page's constant affine. The
// fragment stage takes the map from shape units to device pixels at each
// fragment from the same analytic Jacobian, at the fragment's own
// shape-unit position: exact, and the same on every adapter. Derivatives
// across a 2x2 quad would not do: `dpdx` may be fine or coarse, at the
// implementation's choice, and either way it is the Jacobian half a pixel
// or more from the fragment, an error of first order in the pixel step
// (#422's review: 0.046 of coverage at the band's edge at 60 degrees on
// a coarse adapter, 0.116 at 75).

@vertex
fn vs_main(@builtin(vertex_index) corner: u32, t: Triangle, item: Item) -> Out {
    let at = corner_of(t, corner);
    let j = screen_jacobian(item.linear, item.place.xy, at);
    let det = j.x * j.w - j.y * j.z;
    var moved = vec2(0.0);
    // Edge-on (the columns parallel, as closed_forms_3d.wgsl's padding
    // takes it, independent of the view box's scale) or behind the eye
    // (j zero): no move.
    if (abs(det) > 1e-6 * length(j.xy) * length(j.zw)) {
        moved = local_move(corner_normals(t, corner), corner_kind(t, corner), j, 1.0 / det);
    }
    let local = at + moved;
    let position = to_clip() * vec4(on_plane(item.linear, item.place.xy, local), 0.0, 1.0);
    return vertex_out(t, item, local, position, vec4(0.0));
}

@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    let l = screen_jacobian(in.linear, in.shift, in.local);
    let det = l.x * l.w - l.y * l.z;
    // A fragment exists only in front of the eye (the rasteriser clips the
    // rest), so the Jacobian is zero or singular only edge-on, where the
    // plane is a line on the screen: nothing is drawn there.
    if (det == 0.0) {
        return vec4(in.colour.rgb, 0.0);
    }
    return covered(in, inverse_transpose(l, 1.0 / det), l);
}
