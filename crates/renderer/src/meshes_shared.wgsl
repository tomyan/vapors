// Mesh items, the part shared by the page's stages (meshes_2d.wgsl) and
// the scene's (meshes_3d.wgsl): an entry's triangles, one instance each,
// every corner with its wedge (crates/shapes/src/mesh.rs: the outward
// unit normals of the two outline edges bounding it, how far each edge
// runs from it, and whether it is convex (1), reflex (-1) or inside
// (0)), under a map from the mesh's units to device pixels: the item's
// affine on the page, a plane's projection in the scene. The vertex
// stage picks its corner by vertex index and moves an outline corner
// half a device pixel out across both its lines in device space (the
// device miter, clamped at SVG's limit of 4). The fragment stage takes
// coverage as 0.5 plus the least, over the triangle's outline corners,
// of the corner's wedge distance in device pixels (`wedge`), a line
// counting only alongside its edge, not past the edge's far end: 0.5 on
// the outline, the ramp half a pixel either side. A line's distance in
// device pixels is its distance in the mesh's units over the length of
// the map's inverse transpose applied to its normal: the same for the
// whole item on the page, the map at each fragment in the scene.

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
    // the item's translation (e, f), which the scene's fragment stage
    // reads to take its Jacobian
    @location(11) @interpolate(flat) shift: vec2<f32>,
}

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

// What the fragment stage reads of corner `corner` of triangle `t`, at
// `local` (shape units) placed at `position`; `inverse_transpose` and
// `linear` are the item's, which the page's fragment stage reads.
fn vertex_out(t: Triangle, item: Item, local: vec2<f32>, position: vec4<f32>, inverse_transpose: vec4<f32>) -> Out {
    var out: Out;
    out.position = position;
    out.local = local;
    out.normals0 = t.normals0;
    out.at0 = vec4(t.position0, t.kind0, t.reaches0.x);
    out.normals1 = t.normals1;
    out.at1 = vec4(t.position1, t.kind1, t.reaches1.x);
    out.normals2 = t.normals2;
    out.at2 = vec4(t.position2, t.kind2, t.reaches2.x);
    out.second_reaches = vec4(t.reaches0.y, t.reaches1.y, t.reaches2.y, 0.0);
    out.inverse_transpose = inverse_transpose;
    out.colour = item.colour;
    out.linear = item.linear;
    out.shift = item.place.xy;
    return out;
}

// Corner `corner` of triangle `t`: its position and its move's inputs.
fn corner_of(t: Triangle, corner: u32) -> vec2<f32> {
    let positions = array(t.position0, t.position1, t.position2);
    return positions[corner];
}

fn corner_normals(t: Triangle, corner: u32) -> vec4<f32> {
    let normals = array(t.normals0, t.normals1, t.normals2);
    return normals[corner];
}

fn corner_kind(t: Triangle, corner: u32) -> f32 {
    let kinds = array(t.kind0, t.kind1, t.kind2);
    return kinds[corner];
}

// How far p lies along the edge from `corner` with outward unit normal
// m, whose edge runs `reach` along (-m.y, m.x): negative behind the
// corner, past `abs(reach)` beyond the edge's far end.
fn along(m: vec2<f32>, reach: f32, corner: vec2<f32>, p: vec2<f32>) -> f32 {
    return dot(p - corner, vec2(-m.y, m.x)) * sign(reach);
}

// One line's distance at p in device pixels, inside positive: the line
// through `corner` with outward unit normal m, whose edge runs `reach`
// along (-m.y, m.x). Past the edge's far end the line does not limit
// the shape: FAR, so a convex corner's other line decides and a reflex
// corner limits nothing there.
fn line(m: vec2<f32>, reach: f32, corner: vec2<f32>, p: vec2<f32>, t: vec4<f32>) -> f32 {
    if (along(m, reach, corner, p) > abs(reach)) {
        return FAR;
    }
    return -dot(m, p - corner) / max(length(device_normal(m, t)), 1e-20);
}

// `v`, a vector in the mesh's units, on the device under `l`, the map
// [[a, c], [b, d]] from mesh units to device pixels.
fn on_device_vector(v: vec2<f32>, l: vec4<f32>) -> vec2<f32> {
    return vec2(l.x * v.x + l.z * v.y, l.y * v.x + l.w * v.y);
}

// Whether p lies behind `corner` along the edge with outward unit normal
// m running `reach` along (-m.y, m.x), measured on the device: under a
// non-uniform scale or a skew, the line through the corner perpendicular
// to the edge in mesh units is not perpendicular on the device, and the
// corner's region is the device's (#420's review, L1).
fn behind_on_device(m: vec2<f32>, reach: f32, q: vec2<f32>, l: vec4<f32>) -> bool {
    let along = on_device_vector(vec2(-m.y, m.x) * sign(reach), l);
    return dot(on_device_vector(q, l), along) <= 0.0;
}

// A corner's wedge distance at p in device pixels, inside positive. A
// convex corner's is the nearer of its two lines, except outside both
// lines and behind the corner along both its edges, where it is the
// device distance to the corner, as the closed forms take it; a reflex
// corner's, mirrored, is the farther of its lines, except inside both
// and behind the corner along both edges, where it is the distance to
// the corner (inside the notched rect's notch corner, #420's review
// round: the lines alone gave 0.25 px where the corner is 0.35 away). Outside
// both lines alone is not enough: beside a corner that turns little, a
// point just outside one edge is outside the other line too, extended
// back past the corner, and its nearest point is on the edge (the Redo
// icon's arc join). Past an acute tip, a point behind both edges but
// inside one line keeps the other line's distance, not the tip's: the
// tip stays mitred, as the outline is, rather than rounded. With the
// test on the device, each distance is the device distance under any
// affine map, a non-uniform scale or a skew included.
fn wedge(n: vec4<f32>, at: vec4<f32>, second_reach: f32, p: vec2<f32>, t: vec4<f32>, l: vec4<f32>) -> f32 {
    if (at.z == 0.0) {
        return FAR;
    }
    let convex = at.z > 0.0;
    let d0 = line(n.xy, at.w, at.xy, p, t);
    let d1 = line(n.zw, second_reach, at.xy, p, t);
    let q = p - at.xy;
    let behind = behind_on_device(n.xy, at.w, q, l) && behind_on_device(n.zw, second_reach, q, l);
    if (convex && behind && d0 < 0.0 && d1 < 0.0) {
        return -length(on_device_vector(q, l));
    }
    if (!convex && behind && d0 > 0.0 && d1 > 0.0) {
        return length(on_device_vector(q, l));
    }
    return select(max(d0, d1), min(d0, d1), convex);
}

// The colour of `in` with its coverage at its position, `t` the inverse
// transpose of the shape-to-device linear map there and `l` the map.
fn covered(in: Out, t: vec4<f32>, l: vec4<f32>) -> vec4<f32> {
    let w0 = wedge(in.normals0, in.at0, in.second_reaches.x, in.local, t, l);
    let w1 = wedge(in.normals1, in.at1, in.second_reaches.y, in.local, t, l);
    let w2 = wedge(in.normals2, in.at2, in.second_reaches.z, in.local, t, l);
    let coverage = clamp(0.5 + min(w0, min(w1, w2)), 0.0, 1.0);
    return vec4(in.colour.rgb, in.colour.a * coverage);
}
