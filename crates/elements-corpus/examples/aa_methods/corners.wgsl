// The exact-corner candidate for S3b (one-scene 1.2's known limit),
// a shader-only prototype: each triangle carries, flat, its three
// corners' positions, miter offsets and convex (1), reflex (-1) or
// inside (0) flags. Coverage is 0.5 plus the least, over its outline
// corners, of the corner's wedge distance in device pixels (convex:
// the nearer of its two lines; reflex: the farther), each line's
// distance taken from the interpolated position in the mesh's units
// and divided by its screen gradient. The vertex move is the mesh
// pipeline's, so fragments reach half a pixel past the outline.

struct Vertex {
    @location(0) position: vec2<f32>,
    @location(1) normal: vec2<f32>,
    @location(2) c0: vec4<f32>,
    @location(3) c1: vec4<f32>,
    @location(4) c2: vec4<f32>,
    @location(5) flags: vec4<f32>,
    @location(6) linear: vec4<f32>,
    @location(7) place: vec4<f32>,
    @location(8) view: vec4<f32>,
    @location(9) colour: vec4<f32>,
}

struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) @interpolate(flat) c0: vec4<f32>,
    @location(2) @interpolate(flat) c1: vec4<f32>,
    @location(3) @interpolate(flat) c2: vec4<f32>,
    @location(4) @interpolate(flat) flags: vec4<f32>,
    @location(5) @interpolate(flat) colour: vec4<f32>,
}

const HALF: f32 = 0.5;
const MITER_LIMIT: f32 = 4.0;

fn edge_normals(n: vec2<f32>) -> array<vec2<f32>, 2> {
    let length2 = dot(n, n);
    let across = sqrt(max(length2 - 1.0, 0.0));
    let side = vec2(-n.y, n.x) * across;
    return array((n + side) / length2, (n - side) / length2);
}
fn on_device(m: vec2<f32>, l: vec4<f32>, inverse: f32) -> vec2<f32> {
    // The inverse transpose of [[a, c], [b, d]].
    return normalize(vec2(l.w * m.x - l.y * m.y, -l.z * m.x + l.x * m.y) * inverse);
}
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
fn vs_main(v: Vertex) -> Out {
    let l = v.linear;
    let local = v.position + local_move(v.normal, l, v.place.z);
    let pixel = vec2(l.x * local.x + l.z * local.y, l.y * local.x + l.w * local.y) + v.place.xy;
    var out: Out;
    out.position = vec4(pixel.x / v.view.x * 2.0 - 1.0, 1.0 - pixel.y / v.view.y * 2.0, 0.0, 1.0);
    out.local = local;
    out.c0 = v.c0;
    out.c1 = v.c1;
    out.c2 = v.c2;
    out.flags = v.flags;
    out.colour = v.colour;
    return out;
}

// Corner `c` (position, miter offset) of kind `flag`: its wedge's
// distance in device pixels at `p`, inside positive; far for none.
fn wedge(c: vec4<f32>, flag: f32, p: vec2<f32>, across_x: vec2<f32>, across_y: vec2<f32>) -> f32 {
    if (flag == 0.0) {
        return 1e9;
    }
    let m = edge_normals(c.zw);
    var d: array<f32, 2>;
    for (var i = 0; i < 2; i++) {
        let along = -dot(m[i], p - c.xy);
        let rate = max(length(vec2(dot(m[i], across_x), dot(m[i], across_y))), 1e-20);
        d[i] = along / rate;
    }
    return select(max(d[0], d[1]), min(d[0], d[1]), flag > 0.0);
}

@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    let across_x = dpdx(in.local);
    let across_y = dpdy(in.local);
    let w0 = wedge(in.c0, in.flags.x, in.local, across_x, across_y);
    let w1 = wedge(in.c1, in.flags.y, in.local, across_x, across_y);
    let w2 = wedge(in.c2, in.flags.z, in.local, across_x, across_y);
    let coverage = clamp(0.5 + min(w0, min(w1, w2)), 0.0, 1.0);
    return vec4(in.colour.rgb, in.colour.a * coverage);
}
