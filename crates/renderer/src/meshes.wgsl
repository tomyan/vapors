// Mesh items: an entry's triangles, one instance each, every corner
// with its wedge (crates/shapes/src/mesh.rs: the outward unit normals
// of the two outline edges bounding it and whether it is convex (1),
// reflex (-1) or inside (0)), under an item's affine transform. The
// vertex shader picks its corner by vertex index and moves an outline
// corner half a device pixel out across both its lines in device space
// (the device miter, clamped at SVG's limit of 4). The fragment shader
// takes coverage as 0.5 plus the least, over the triangle's outline
// corners, of the corner's wedge distance in device pixels (convex: the
// nearer of its two lines; reflex: the farther): 0.5 on the outline,
// the ramp half a pixel either side, round every corner as Chromium
// draws it. Each line's distance is affine across the triangle, so its
// screen gradient from dpdx/dpdy of the position is exact.

struct Triangle {
    @location(0) position0: vec2<f32>,
    @location(1) normals0: vec4<f32>,
    @location(2) kind0: f32,
    @location(3) position1: vec2<f32>,
    @location(4) normals1: vec4<f32>,
    @location(5) kind1: f32,
    @location(6) position2: vec2<f32>,
    @location(7) normals2: vec4<f32>,
    @location(8) kind2: f32,
}

struct Item {
    @location(9) linear: vec4<f32>, // a, b, c, d of matrix(a, b, c, d, e, f)
    @location(10) place: vec4<f32>, // e, f, 1 / (a d - b c), unused
    @location(11) colour: vec4<f32>,
}

// Each corner as two vec4s: its normals, then its position and kind.
struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) @interpolate(flat) normals0: vec4<f32>,
    @location(2) @interpolate(flat) at0: vec4<f32>,
    @location(3) @interpolate(flat) normals1: vec4<f32>,
    @location(4) @interpolate(flat) at1: vec4<f32>,
    @location(5) @interpolate(flat) normals2: vec4<f32>,
    @location(6) @interpolate(flat) at2: vec4<f32>,
    @location(7) @interpolate(flat) colour: vec4<f32>,
}

struct View {
    size: vec2<f32>,
    shift: vec2<f32>,
}

@group(0) @binding(0) var<uniform> view: View;

// How far the outline moves out, device pixels: the ramp's outer half.
const HALF: f32 = 0.5;
// SVG's miter limit: a corner moves at most this many times HALF.
const MITER_LIMIT: f32 = 4.0;
// A wedge distance no fragment reaches: an inside corner's.
const FAR: f32 = 1e9;

// m, a normal in the mesh's units, as a unit normal on the device.
fn on_device(m: vec2<f32>, l: vec4<f32>, inverse: f32) -> vec2<f32> {
    // The inverse transpose of [[a, c], [b, d]].
    return normalize(vec2(l.w * m.x - l.y * m.y, -l.z * m.x + l.x * m.y) * inverse);
}

// The move, in the mesh's units, that carries a corner with normals n
// and kind `kind` half a device pixel out across both its lines.
fn local_move(n: vec4<f32>, kind: f32, l: vec4<f32>, inverse: f32) -> vec2<f32> {
    if (kind == 0.0) {
        return vec2(0.0);
    }
    let u0 = on_device(n.xy, l, inverse);
    let u1 = on_device(n.zw, l, inverse);
    var miter = (u0 + u1) / max(1.0 + dot(u0, u1), 1e-6) * HALF;
    let limit = MITER_LIMIT * HALF;
    if (length(miter) > limit) {
        miter = normalize(miter) * limit;
    }
    // The inverse of [[a, c], [b, d]].
    return vec2(l.w * miter.x - l.z * miter.y, -l.y * miter.x + l.x * miter.y) * inverse;
}

@vertex
fn vs_main(@builtin(vertex_index) corner: u32, t: Triangle, item: Item) -> Out {
    let positions = array(t.position0, t.position1, t.position2);
    let normals = array(t.normals0, t.normals1, t.normals2);
    let kinds = array(t.kind0, t.kind1, t.kind2);
    let l = item.linear;
    let local = positions[corner] + local_move(normals[corner], kinds[corner], l, item.place.z);
    let pixel = vec2(l.x * local.x + l.z * local.y, l.y * local.x + l.w * local.y) + item.place.xy;
    let on_target = pixel + view.shift;
    var out: Out;
    out.position = vec4(on_target.x / view.size.x * 2.0 - 1.0, 1.0 - on_target.y / view.size.y * 2.0, 0.0, 1.0);
    out.local = local;
    out.normals0 = t.normals0;
    out.at0 = vec4(t.position0, t.kind0, 0.0);
    out.normals1 = t.normals1;
    out.at1 = vec4(t.position1, t.kind1, 0.0);
    out.normals2 = t.normals2;
    out.at2 = vec4(t.position2, t.kind2, 0.0);
    out.colour = item.colour;
    return out;
}

// One line's distance at p in device pixels, inside positive: the line
// through `corner` with outward unit normal m.
fn line(m: vec2<f32>, corner: vec2<f32>, p: vec2<f32>, across_x: vec2<f32>, across_y: vec2<f32>) -> f32 {
    let rate = max(length(vec2(dot(m, across_x), dot(m, across_y))), 1e-20);
    return -dot(m, p - corner) / rate;
}

// A corner's wedge distance at p in device pixels, inside positive.
fn wedge(n: vec4<f32>, at: vec4<f32>, p: vec2<f32>, across_x: vec2<f32>, across_y: vec2<f32>) -> f32 {
    if (at.z == 0.0) {
        return FAR;
    }
    let d0 = line(n.xy, at.xy, p, across_x, across_y);
    let d1 = line(n.zw, at.xy, p, across_x, across_y);
    return select(max(d0, d1), min(d0, d1), at.z > 0.0);
}

@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    let across_x = dpdx(in.local);
    let across_y = dpdy(in.local);
    let w0 = wedge(in.normals0, in.at0, in.local, across_x, across_y);
    let w1 = wedge(in.normals1, in.at1, in.local, across_x, across_y);
    let w2 = wedge(in.normals2, in.at2, in.local, across_x, across_y);
    let coverage = clamp(0.5 + min(w0, min(w1, w2)), 0.0, 1.0);
    return vec4(in.colour.rgb, in.colour.a * coverage);
}
