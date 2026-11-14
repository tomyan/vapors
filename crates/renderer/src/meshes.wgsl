// Mesh items: an entry's triangles, one instance each, every corner
// with its wedge (crates/shapes/src/mesh.rs: the outward unit normals
// of the two outline edges bounding it, how far each edge runs from it,
// and whether it is convex (1), reflex (-1) or inside (0)), under an
// item's affine transform. The vertex shader picks its corner by vertex
// index and moves an outline corner half a device pixel out across both
// its lines in device space (the device miter, clamped at SVG's limit
// of 4). The fragment shader takes coverage as 0.5 plus the least, over
// the triangle's outline corners, of the corner's wedge distance in
// device pixels (convex: the nearer of its two lines, or beyond both
// the device distance to the corner; reflex: the farther line), a line
// counting only alongside its edge, not past the edge's far end: 0.5
// on the outline, the ramp half a pixel either side. A line's distance in device pixels is its distance in the
// mesh's units over the length of the inverse transpose of the
// transform applied to its normal, the same for the whole item.

struct Triangle {
    @location(0) position0: vec2<f32>,
    @location(1) normals0: vec4<f32>,
    @location(2) kind0: f32,
    @location(3) reaches0: vec2<f32>,
    @location(4) position1: vec2<f32>,
    @location(5) normals1: vec4<f32>,
    @location(6) kind1: f32,
    @location(7) reaches1: vec2<f32>,
    @location(8) position2: vec2<f32>,
    @location(9) normals2: vec4<f32>,
    @location(10) kind2: f32,
    @location(11) reaches2: vec2<f32>,
}

struct Item {
    @location(12) linear: vec4<f32>, // a, b, c, d of matrix(a, b, c, d, e, f)
    @location(13) place: vec4<f32>, // e, f, 1 / (a d - b c), unused
    @location(14) colour: vec4<f32>,
}

// Each corner as two vec4s, its normals, then its position, kind and
// first reach; the three second reaches together; the inverse
// transpose of the transform's linear part; the colour; the linear
// part itself.
struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) @interpolate(flat) normals0: vec4<f32>,
    @location(2) @interpolate(flat) at0: vec4<f32>,
    @location(3) @interpolate(flat) normals1: vec4<f32>,
    @location(4) @interpolate(flat) at1: vec4<f32>,
    @location(5) @interpolate(flat) normals2: vec4<f32>,
    @location(6) @interpolate(flat) at2: vec4<f32>,
    @location(7) @interpolate(flat) second_reaches: vec4<f32>,
    @location(8) @interpolate(flat) inverse_transpose: vec4<f32>,
    @location(9) @interpolate(flat) colour: vec4<f32>,
    @location(10) @interpolate(flat) linear: vec4<f32>,
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

// The inverse transpose of [[a, c], [b, d]], row by row.
fn inverse_transpose(l: vec4<f32>, inverse: f32) -> vec4<f32> {
    return vec4(l.w, -l.y, -l.z, l.x) * inverse;
}

// m, a normal in the mesh's units, on the device: its length is the
// device pixels a mesh unit along m spans, inverted.
fn device_normal(m: vec2<f32>, t: vec4<f32>) -> vec2<f32> {
    return vec2(t.x * m.x + t.y * m.y, t.z * m.x + t.w * m.y);
}

// m, a normal in the mesh's units, as a unit normal on the device.
fn on_device(m: vec2<f32>, l: vec4<f32>, inverse: f32) -> vec2<f32> {
    return normalize(device_normal(m, inverse_transpose(l, inverse)));
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
    out.at0 = vec4(t.position0, t.kind0, t.reaches0.x);
    out.normals1 = t.normals1;
    out.at1 = vec4(t.position1, t.kind1, t.reaches1.x);
    out.normals2 = t.normals2;
    out.at2 = vec4(t.position2, t.kind2, t.reaches2.x);
    out.second_reaches = vec4(t.reaches0.y, t.reaches1.y, t.reaches2.y, 0.0);
    out.inverse_transpose = inverse_transpose(l, item.place.z);
    out.colour = item.colour;
    out.linear = l;
    return out;
}

// One line's distance at p in device pixels, inside positive: the line
// through `corner` with outward unit normal m, whose edge runs `reach`
// along (-m.y, m.x). Past the edge's far end the line does not limit
// the shape: FAR, so a convex corner's other line decides and a reflex
// corner limits nothing there.
fn line(m: vec2<f32>, reach: f32, corner: vec2<f32>, p: vec2<f32>, t: vec4<f32>) -> f32 {
    let along = dot(p - corner, vec2(-m.y, m.x)) * sign(reach);
    if (along > abs(reach)) {
        return FAR;
    }
    return -dot(m, p - corner) / max(length(device_normal(m, t)), 1e-20);
}

// A corner's wedge distance at p in device pixels, inside positive:
// beyond both lines of a convex corner, the device distance to the
// corner, as the closed forms take it.
fn wedge(n: vec4<f32>, at: vec4<f32>, second_reach: f32, p: vec2<f32>, t: vec4<f32>, l: vec4<f32>) -> f32 {
    if (at.z == 0.0) {
        return FAR;
    }
    let convex = at.z > 0.0;
    let d0 = line(n.xy, at.w, at.xy, p, t);
    let d1 = line(n.zw, second_reach, at.xy, p, t);
    if (convex && d0 < 0.0 && d1 < 0.0) {
        let q = p - at.xy;
        return -length(vec2(l.x * q.x + l.z * q.y, l.y * q.x + l.w * q.y));
    }
    return select(max(d0, d1), min(d0, d1), convex);
}

@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    let t = in.inverse_transpose;
    let w0 = wedge(in.normals0, in.at0, in.second_reaches.x, in.local, t, in.linear);
    let w1 = wedge(in.normals1, in.at1, in.second_reaches.y, in.local, t, in.linear);
    let w2 = wedge(in.normals2, in.at2, in.second_reaches.z, in.local, t, in.linear);
    let coverage = clamp(0.5 + min(w0, min(w1, w2)), 0.0, 1.0);
    return vec4(in.colour.rgb, in.colour.a * coverage);
}
