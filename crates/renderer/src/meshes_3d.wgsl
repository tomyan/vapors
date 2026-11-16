// The scene's stages for mesh items (the rest is meshes_shared.wgsl and
// projective.wgsl; docs/plans/decal.md section 2). A plane's projection
// keeps lines straight, so the vertex stage carries each corner's wedge
// onto the screen by projecting the corner and its edges' far ends, and
// the fragment stage measures every distance on the screen, exactly, at
// any angle. The vertex stage also moves each outline corner half a
// device pixel out through the screen Jacobian at that corner.

// Mesh point `q` (shape units) in clip space under the item and node.
fn clip_of(item: Item, q: vec2<f32>) -> vec4<f32> {
    return to_clip() * vec4(on_plane(item.linear, item.place.xy, q), 0.0, 1.0);
}

// Clip point `c` on the screen, framebuffer pixels, +Y down.
fn on_screen(c: vec4<f32>) -> vec2<f32> {
    return (c.xy / c.w * vec2(0.5, -0.5) + 0.5) * camera.size;
}

// The edge from `at` along local `edge` on the screen, from the corner's
// own screen point `start`: its screen vector, or, when its far end is at
// or behind the eye, the corner's Jacobian applied to it stretched past
// any fragment (FAR), so its line's direction holds and no far end cuts
// it short on the screen.
fn screen_edge(item: Item, at: vec2<f32>, edge: vec2<f32>, start: vec2<f32>, j: vec4<f32>) -> vec2<f32> {
    let end = clip_of(item, at + edge);
    if (end.w <= NEAR_W) {
        let along = vec2(j.x * edge.x + j.z * edge.y, j.y * edge.x + j.w * edge.y);
        return normalize(along) * FAR;
    }
    return on_screen(end) - start;
}

// The outward unit normal on the screen of an edge running `edge` (screen
// pixels), on the side the local normal `n` maps to under `j`.
fn screen_normal(edge: vec2<f32>, n: vec2<f32>, j: vec4<f32>) -> vec2<f32> {
    let across = normalize(vec2(edge.y, -edge.x));
    let outward = vec2(j.x * n.x + j.z * n.y, j.y * n.x + j.w * n.y);
    return select(-across, across, dot(across, outward) >= 0.0);
}

// Corner `corner` of `t` on the screen. A corner at or behind the eye
// has no wedge here: its edges' lines are carried by the corners at
// their other ends that are in front, and the rasteriser clips the rest.
fn scene_corner(t: Triangle, item: Item, corner: u32) -> DeviceCorner {
    let kind = corner_kind(t, corner);
    let n = corner_normals(t, corner);
    let at = corner_of(t, corner);
    let c = clip_of(item, at);
    if (kind == 0.0 || c.w <= NEAR_W) {
        return NO_WEDGE;
    }
    let start = on_screen(c);
    let j = screen_jacobian(item.linear, item.place.xy, at);
    let reaches = corner_reaches(t, corner);
    let edge0 = screen_edge(item, at, vec2(-n.y, n.x) * reaches.x, start, j);
    let edge1 = screen_edge(item, at, vec2(-n.w, n.z) * reaches.y, start, j);
    let n0 = screen_normal(edge0, n.xy, j);
    let n1 = screen_normal(edge1, n.zw, j);
    return DeviceCorner(vec4(start, kind, device_reach(edge0, n0)), vec4(n0, n1), device_reach(edge1, n1));
}

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
    let position = clip_of(item, at + moved);
    let corners = array(scene_corner(t, item, 0u), scene_corner(t, item, 1u), scene_corner(t, item, 2u));
    return vertex_out(corners, item.colour, position);
}

@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    return covered(in);
}
