// Mesh items: an entry's triangles, each vertex with its own three edge
// distances (crates/shapes/src/mesh.rs: 0 on an outline edge's corners,
// the altitude opposite, FAR on all three for an edge two triangles
// share) and its triangle's edge normals, under an item's affine
// transform. The vertex shader rebuilds an outline vertex's two edge
// normals from its miter offset, moves it half a device pixel out
// across both in device space (the device miter, clamped at SVG's
// limit of 4), and takes the move's projection on each edge normal off
// that edge's distance, so the distances stay exact for the moved
// triangle and go negative past the true outline. The fragment shader
// takes coverage as 0.5 plus the distance to the nearest outline edge
// in device pixels: 0.5 on the outline, the ramp half a pixel either
// side, as Chromium draws an edge. Each distance is affine across its
// triangle, so its screen gradient from dpdx/dpdy is exact.

struct Vertex {
    @location(0) position: vec2<f32>,
    @location(1) normal: vec2<f32>,
    @location(2) edges: vec3<f32>,
    @location(3) normal0: vec2<f32>,
    @location(4) normal1: vec2<f32>,
    @location(5) normal2: vec2<f32>,
}

struct Instance {
    @location(6) linear: vec4<f32>, // a, b, c, d of matrix(a, b, c, d, e, f)
    @location(7) place: vec4<f32>,  // e, f, 1 / (a d - b c), unused
    @location(8) colour: vec4<f32>,
}

struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) edges: vec3<f32>,
    @location(1) @interpolate(flat) colour: vec4<f32>,
}

struct View {
    size: vec2<f32>,
    shift: vec2<f32>,
}

@group(0) @binding(0) var<uniform> view: View;

// An edge two triangles share carries FAR (1e6) on all three corners;
// a real distance is never within half of it.
const FAR_BELOW: f32 = 5.0e5;
// How far the outline moves out, device pixels: the ramp's outer half.
const HALF: f32 = 0.5;
// SVG's miter limit: a corner moves at most this many times HALF.
const MITER_LIMIT: f32 = 4.0;

// The unit vectors m with n . m = 1: a vertex's two outline edges'
// normals rebuilt from its miter offset n (one, twice, when |n| = 1; a
// clamped miter's are the ones its clamped length would have).
fn edge_normals(n: vec2<f32>) -> array<vec2<f32>, 2> {
    let length2 = dot(n, n);
    let across = sqrt(max(length2 - 1.0, 0.0));
    let side = vec2(-n.y, n.x) * across;
    return array((n + side) / length2, (n - side) / length2);
}

// m, a normal in the mesh's units, as a unit normal on the device.
fn on_device(m: vec2<f32>, l: vec4<f32>, inverse: f32) -> vec2<f32> {
    // The inverse transpose of [[a, c], [b, d]].
    return normalize(vec2(l.w * m.x - l.y * m.y, -l.z * m.x + l.x * m.y) * inverse);
}

// The move, in the mesh's units, that carries a vertex with miter
// offset n half a device pixel out across both its outline edges.
fn local_move(n: vec2<f32>, l: vec4<f32>, inverse: f32) -> vec2<f32> {
    if (dot(n, n) < 1e-12) {
        return vec2(0.0);
    }
    let m = edge_normals(n);
    let u = array(on_device(m[0], l, inverse), on_device(m[1], l, inverse));
    var miter = (u[0] + u[1]) / max(1.0 + dot(u[0], u[1]), 1e-6) * HALF;
    let limit = MITER_LIMIT * HALF;
    if (length(miter) > limit) {
        miter = normalize(miter) * limit;
    }
    // The inverse of [[a, c], [b, d]].
    return vec2(l.w * miter.x - l.z * miter.y, -l.y * miter.x + l.x * miter.y) * inverse;
}

@vertex
fn vs_main(vertex: Vertex, instance: Instance) -> Out {
    let l = instance.linear;
    let inverse = instance.place.z;
    let moved = local_move(vertex.normal, l, inverse);
    let local = vertex.position + moved;
    let pixel = vec2(l.x * local.x + l.z * local.y, l.y * local.x + l.w * local.y) + instance.place.xy;
    let on_target = pixel + view.shift;
    var out: Out;
    out.position = vec4(on_target.x / view.size.x * 2.0 - 1.0, 1.0 - on_target.y / view.size.y * 2.0, 0.0, 1.0);
    let taken = vec3(dot(moved, vertex.normal0), dot(moved, vertex.normal1), dot(moved, vertex.normal2));
    out.edges = vertex.edges - taken;
    out.colour = instance.colour;
    return out;
}

@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    let across = sqrt(dpdx(in.edges) * dpdx(in.edges) + dpdy(in.edges) * dpdy(in.edges));
    let far = in.edges >= vec3(FAR_BELOW);
    let pixels = select(in.edges / max(across, vec3(1e-20)), vec3(1e9), far);
    let nearest = min(pixels.x, min(pixels.y, pixels.z));
    let coverage = clamp(0.5 + nearest, 0.0, 1.0);
    return vec4(in.colour.rgb, in.colour.a * coverage);
}
