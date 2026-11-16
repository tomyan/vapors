// The scene's vertex stage for closed-form shapes (the rest is
// closed_forms_shared.wgsl and projective.wgsl; docs/plans/decal.md
// section 2): the instance's affine transform takes shape units to a
// node's plane, z = 0, and the node's world matrix and the camera's
// view-projection take the plane to clip space. The fragment stage is
// the page's: it reads the distance's rate across the screen from the
// derivatives of the shape-unit position, which under a projection is
// interpolated perspective-correct. Taken across a 2x2 quad, those
// derivatives are the local Jacobian with an error of second order in
// the quad's size: exact under an affine map, and here within the
// bounds tests/gpu_closed_forms_3d.rs measured against a CPU drawing.

// The padding in shape units along x and y that moves each edge of the
// box MARGIN device pixels out at `local`, as the page's `padding` does
// with the screen Jacobian there in place of the affine's. Lines stay
// lines on a plane, so an edge padded at both its corners stays about
// the smaller margin from the shape: to first order, since each pad is
// set from the Jacobian at the unpadded corner (a plane within a tenth
// of a degree of edge-on can leave 0.3 px, D1 review L2; there the shape
// is a sub-pixel sliver).
fn projective_padding(instance: Instance, local: vec2<f32>) -> vec2<f32> {
    let j = screen_jacobian(instance.linear, instance.shift.xy, local);
    // Edge-on the pad is unbounded (`edge_on`).
    if (edge_on(j)) {
        return vec2(0.0);
    }
    let det = abs(j.x * j.w - j.y * j.z);
    return MARGIN * vec2(length(j.zw), length(j.xy)) / det;
}

@vertex
fn vs_main(@builtin(vertex_index) vertex: u32, instance: Instance) -> Out {
    let box = local_box(instance);
    let at = quad_corner(vertex);
    let corner = mix(box.xy, box.zw, at);
    let pad = projective_padding(instance, corner);
    let local = corner + (at * 2.0 - 1.0) * pad;
    let position = to_clip() * vec4(on_plane(instance.linear, instance.shift.xy, local), 0.0, 1.0);
    return vertex_out(instance, local, position);
}
