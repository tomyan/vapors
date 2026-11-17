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
    // Edge-on the pad is unbounded (`edge_on`).
    if (edge_on(j)) {
        return vec2(0.0);
    }
    let det = abs(j.x * j.w - j.y * j.z);
    return MARGIN * vec2(length(j.zw), length(j.xy)) / det;
}

@vertex
fn vs_main(@builtin(vertex_index) vertex: u32, instance: Instance) -> SceneOut {
    let box = local_box(instance);
    let at = quad_corner(vertex);
    let corner = mix(box.xy, box.zw, at);
    let pad = projective_padding(instance, corner);
    let local = corner + (at * 2.0 - 1.0) * pad;
    let position = to_clip() * vec4(on_plane(instance.linear, instance.shift.xy, local), 0.0, 1.0);
    return scene_out(vertex_out(instance, local, position), instance);
}

// What the scene's fragment stage reads: the page's `Out`, and the
// instance's shape-to-clip columns (`shape_to_clip`), from which it takes
// its Jacobian; the page's stages do not carry them.
struct SceneOut {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) @interpolate(flat) params: vec3<f32>,
    @location(2) @interpolate(flat) form: vec4<f32>,
    @location(3) @interpolate(flat) colour: vec4<f32>,
    @location(4) @interpolate(flat) radii: vec2<f32>,
    @location(5) @interpolate(flat) to_clip_x: vec4<f32>,
    @location(6) @interpolate(flat) to_clip_y: vec4<f32>,
    @location(7) @interpolate(flat) to_clip_origin: vec4<f32>,
}

fn scene_out(page: Out, instance: Instance) -> SceneOut {
    let k = shape_to_clip(instance.linear, instance.shift.xy);
    return SceneOut(page.position, page.local, page.params, page.form, page.colour, page.radii, k[0], k[1], k[2]);
}

fn page_out(in: SceneOut) -> Out {
    // no clip and row 0: a scene's shapes are not clipped by the page's clip table
    return Out(in.position, in.local, in.params, in.form, in.colour, in.radii, 0u, 0u);
}

// The shape units per device pixel, right and down, from the analytic
// Jacobian at the fragment's own position: exact, and the same on every
// adapter. Derivatives across a 2x2 quad would not do under a
// projection: `dpdx` may be fine or coarse, at the implementation's
// choice, and either way it is the Jacobian half a pixel or more from
// the fragment, an error of first order in the pixel step (#422's
// review; modelled at D1's 60 degree pose, the circle and frame of
// gpu_closed_forms_3d_distance.rs at 3 units: 0.027 of coverage at the
// band's edge with fine derivatives, 0.041 with coarse). No guard: a
// fragment exists only where the plane has area on the screen and in
// front of the eye, so its Jacobian is never `edge_on` and 1 / det is
// finite; the edge-on case is the vertex stage's.
@fragment
fn fs_main(in: SceneOut) -> @location(0) vec4<f32> {
    let j = jacobian_at(mat3x4(in.to_clip_x, in.to_clip_y, in.to_clip_origin), in.local);
    let det = j.x * j.w - j.y * j.z;
    return covered(page_out(in), vec2(j.w, -j.y) / det, vec2(-j.z, j.x) / det);
}
