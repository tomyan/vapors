// The scene's vertex stage for closed-form shapes (the rest is
// closed_forms_shared.wgsl and projective.wgsl; docs/plans/decal.md
// section 2): the instance's affine transform takes shape units to a
// node's plane, z = 0, and the node's world matrix and the camera's
// view-projection take the plane to clip space. The fragment stage
// (`fs_main`, below) is the page's coverage with the map from device
// pixels to shape units taken from the analytic Jacobian.

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
    // Edge-on, the columns are parallel and the pad unbounded. The ratio
    // is the sine of the angle between them, independent of the view
    // box's scale: under 1e-6 the plane is within 6e-5 degrees of
    // edge-on, a sliver under a millionth of the shape's width on screen.
    let det = abs(j.x * j.w - j.y * j.z);
    if (det <= 1e-6 * length(j.xy) * length(j.zw)) {
        return vec2(0.0);
    }
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

// The shape units per device pixel, right and down, from the analytic
// Jacobian at the fragment's own position: exact, and the same on every
// adapter. Derivatives across a 2x2 quad would not do under a
// projection: `dpdx` may be fine or coarse, at the implementation's
// choice, and either way it is the Jacobian half a pixel or more from
// the fragment, an error of first order in the pixel step (#422's
// review; modelled at D1's 60 degree pose: 0.027 of coverage at the
// band's edge with fine derivatives, 0.041 with coarse). A fragment
// exists only in front of the eye, so the Jacobian is singular only
// edge-on, where the plane is a line on the screen: nothing is drawn.
@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    let j = screen_jacobian(in.linear, in.shift, in.local);
    let det = j.x * j.w - j.y * j.z;
    if (det == 0.0) {
        return vec4(in.colour.rgb, 0.0);
    }
    return covered(in, vec2(j.w, -j.y) / det, vec2(-j.z, j.x) / det);
}
