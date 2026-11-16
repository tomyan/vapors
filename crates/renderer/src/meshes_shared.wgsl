// Mesh items, the part shared by the page's stages (meshes_2d.wgsl) and
// the scene's (meshes_3d.wgsl): an entry's triangles, one instance each,
// every corner with its wedge (crates/shapes/src/mesh.rs: the outward
// unit normals of the two outline edges bounding it, how far each edge
// runs from it, and whether it is convex (1), reflex (-1) or inside
// (0)), under a map from the mesh's units to device pixels: the item's
// affine on the page, a plane's projection in the scene. Both maps take
// lines to lines, so each stage carries its triangle's wedges onto the
// device in its vertex stage (a `DeviceCorner` each: position, unit
// normals and reaches in device pixels) and the fragment stage measures
// every distance there, exactly, from the fragment's own position. The
// vertex stage also moves each outline corner half a device pixel out
// across both its lines (the device miter, clamped at SVG's limit of 4).
// Coverage is 0.5 plus the least, over the triangle's outline corners,
// of the corner's wedge distance (`wedge`), a line counting only
// alongside its edge, not past the edge's far end: 0.5 on the outline,
// the ramp half a pixel either side.

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

// Each corner on the device as two vec4s, its unit normals, then its
// position, kind and first reach; the three second reaches together;
// the colour.
struct Out {
    @builtin(position) position: vec4<f32>,
    @location(1) @interpolate(flat) normals0: vec4<f32>,
    @location(2) @interpolate(flat) at0: vec4<f32>,
    @location(3) @interpolate(flat) normals1: vec4<f32>,
    @location(4) @interpolate(flat) at1: vec4<f32>,
    @location(5) @interpolate(flat) normals2: vec4<f32>,
    @location(6) @interpolate(flat) at2: vec4<f32>,
    @location(7) @interpolate(flat) second_reaches: vec4<f32>,
    @location(9) @interpolate(flat) colour: vec4<f32>,
}

// A corner's wedge on the device: its position, kind and first reach;
// its two outward unit normals; its second reach. Device pixels, +Y down,
// in the framebuffer's coordinates, as a fragment's position is.
struct DeviceCorner {
    at: vec4<f32>,
    normals: vec4<f32>,
    second: f32,
}

// An inside corner: no wedge, so its lines limit nothing.
const NO_WEDGE: DeviceCorner = DeviceCorner(vec4(0.0), vec4(0.0), 0.0);

// The signed device reach of edge vector `edge` (device pixels) along
// unit normal n's edge direction (-n.y, n.x).
fn device_reach(edge: vec2<f32>, n: vec2<f32>) -> f32 {
    return dot(edge, vec2(-n.y, n.x));
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

// What the fragment stage reads: the triangle's three corners on the
// device, its colour, and the vertex placed at `position`.
fn vertex_out(corners: array<DeviceCorner, 3>, colour: vec4<f32>, position: vec4<f32>) -> Out {
    var out: Out;
    out.position = position;
    out.normals0 = corners[0].normals;
    out.at0 = corners[0].at;
    out.normals1 = corners[1].normals;
    out.at1 = corners[1].at;
    out.normals2 = corners[2].normals;
    out.at2 = corners[2].at;
    out.second_reaches = vec4(corners[0].second, corners[1].second, corners[2].second, 0.0);
    out.colour = colour;
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

fn corner_reaches(t: Triangle, corner: u32) -> vec2<f32> {
    let reaches = array(t.reaches0, t.reaches1, t.reaches2);
    return reaches[corner];
}

// How far p lies along the edge from `corner` with outward unit normal
// m, whose edge runs `reach` along (-m.y, m.x): negative behind the
// corner, past `abs(reach)` beyond the edge's far end. Device pixels.
fn along(m: vec2<f32>, reach: f32, corner: vec2<f32>, p: vec2<f32>) -> f32 {
    return dot(p - corner, vec2(-m.y, m.x)) * sign(reach);
}

// One line's distance at p, device pixels, inside positive: the line
// through `corner` with outward unit normal m, whose edge runs `reach`
// along (-m.y, m.x). Past the edge's far end the line does not limit
// the shape: FAR, so a convex corner's other line decides and a reflex
// corner limits nothing there.
fn line(m: vec2<f32>, reach: f32, corner: vec2<f32>, p: vec2<f32>) -> f32 {
    if (along(m, reach, corner, p) > abs(reach)) {
        return FAR;
    }
    return -dot(m, p - corner);
}

// A corner's wedge distance at p, device pixels, inside positive: the
// distance to the outline near the corner, exactly. A convex corner's is
// the nearer of its two lines, except behind the corner along both its
// edges (a region wholly outside the shape), where the outline's nearest
// point is the corner itself: there it is the distance to the corner,
// as the closed forms take it. A reflex corner's, mirrored, is the
// farther of its lines, except inside both and behind the corner along
// both edges, where it is the distance to the corner. Outside both lines
// alone does not mark the corner's region: beside a corner that turns
// little, a point just outside one edge is outside the other line too,
// extended back past the corner, and its nearest point is on the edge
// (the Redo icon's arc join). Nor is it needed: past an acute tip, a
// point behind both edges can lie inside one line, and the tip's
// distance is still the one to the corner (a mitred line distance there
// over-covers: at 85 degrees a foreshortened right angle left a pixel
// 0.61 px outside at 0.26, D2b). Every quantity is on the device, so
// each distance is exact under any map that keeps lines straight.
fn wedge(n: vec4<f32>, at: vec4<f32>, second_reach: f32, p: vec2<f32>) -> f32 {
    if (at.z == 0.0) {
        return FAR;
    }
    let convex = at.z > 0.0;
    let d0 = line(n.xy, at.w, at.xy, p);
    let d1 = line(n.zw, second_reach, at.xy, p);
    let behind = along(n.xy, at.w, at.xy, p) <= 0.0 && along(n.zw, second_reach, at.xy, p) <= 0.0;
    if (convex && behind) {
        return -length(p - at.xy);
    }
    if (!convex && behind && d0 > 0.0 && d1 > 0.0) {
        return length(p - at.xy);
    }
    return select(max(d0, d1), min(d0, d1), convex);
}

// The colour of `in` with its coverage at its own position.
fn covered(in: Out) -> vec4<f32> {
    let p = in.position.xy;
    let w0 = wedge(in.normals0, in.at0, in.second_reaches.x, p);
    let w1 = wedge(in.normals1, in.at1, in.second_reaches.y, p);
    let w2 = wedge(in.normals2, in.at2, in.second_reaches.z, p);
    let coverage = clamp(0.5 + min(w0, min(w1, w2)), 0.0, 1.0);
    return vec4(in.colour.rgb, in.colour.a * coverage);
}
