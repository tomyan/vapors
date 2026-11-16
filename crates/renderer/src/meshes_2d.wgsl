// The page's stages for mesh items (the rest is meshes_shared.wgsl): the
// item's affine transform takes the mesh's units to device pixels on the
// target, so a corner's wedge goes onto the device by that affine.

struct View {
    size: vec2<f32>,
    shift: vec2<f32>,
}

@group(0) @binding(0) var<uniform> view: View;

// Mesh point `q` on the target, framebuffer pixels.
fn on_target(item: Item, q: vec2<f32>) -> vec2<f32> {
    let l = item.linear;
    return vec2(l.x * q.x + l.z * q.y, l.y * q.x + l.w * q.y) + item.place.xy + view.shift;
}

// Corner `corner` of `t` on the device under the item's affine.
fn page_corner(t: Triangle, item: Item, corner: u32) -> DeviceCorner {
    let kind = corner_kind(t, corner);
    let n = corner_normals(t, corner);
    if (kind == 0.0) {
        return NO_WEDGE;
    }
    let at = corner_of(t, corner);
    let reaches = corner_reaches(t, corner);
    let l = item.linear;
    let n0 = on_device(n.xy, l, item.place.z);
    let n1 = on_device(n.zw, l, item.place.z);
    let edge0 = on_target(item, at + vec2(-n.y, n.x) * reaches.x) - on_target(item, at);
    let edge1 = on_target(item, at + vec2(-n.w, n.z) * reaches.y) - on_target(item, at);
    let first = device_reach(edge0, n0);
    return DeviceCorner(vec4(on_target(item, at), kind, first), vec4(n0, n1), device_reach(edge1, n1));
}

@vertex
fn vs_main(@builtin(vertex_index) corner: u32, t: Triangle, item: Item) -> Out {
    let l = item.linear;
    let local = corner_of(t, corner) + local_move(corner_normals(t, corner), corner_kind(t, corner), l, item.place.z);
    let placed = on_target(item, local);
    let position = vec4(placed.x / view.size.x * 2.0 - 1.0, 1.0 - placed.y / view.size.y * 2.0, 0.0, 1.0);
    let corners = array(page_corner(t, item, 0u), page_corner(t, item, 1u), page_corner(t, item, 2u));
    return vertex_out(corners, item.colour, position);
}

@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    return covered(in);
}
