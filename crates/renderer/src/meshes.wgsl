// Mesh items: an entry's triangles, each vertex with its own three edge
// distances (crates/shapes/src/mesh.rs: 0 on an outline edge's corners,
// the altitude opposite, FAR on all three for an edge two triangles
// share), under an item's affine transform. The vertex shader moves
// each vertex by the item's extrusion along its normal (its miter
// offset, zero inside) and takes extrusion * shift off each distance,
// so the distances stay exact for the moved triangle and go negative
// past the true outline. The fragment shader takes coverage as 0.5
// plus the distance to the nearest outline edge in device pixels: 0.5
// on the outline, the ramp half a pixel either side, as Chromium draws
// an edge. Each distance is affine across its triangle, so its screen
// gradient from dpdx/dpdy is exact.

struct Vertex {
    @location(0) position: vec2<f32>,
    @location(1) normal: vec2<f32>,
    @location(2) edges: vec3<f32>,
    @location(3) shift: vec3<f32>,
}

struct Instance {
    @location(4) linear: vec4<f32>, // a, b, c, d of matrix(a, b, c, d, e, f)
    @location(5) place: vec4<f32>,  // e, f, the extrusion in its own units, unused
    @location(6) colour: vec4<f32>,
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

@vertex
fn vs_main(vertex: Vertex, instance: Instance) -> Out {
    let h = instance.place.z;
    let local = vertex.position + h * vertex.normal;
    let m = instance.linear;
    let pixel = vec2(m.x * local.x + m.z * local.y, m.y * local.x + m.w * local.y) + instance.place.xy;
    let on_target = pixel + view.shift;
    var out: Out;
    out.position = vec4(on_target.x / view.size.x * 2.0 - 1.0, 1.0 - on_target.y / view.size.y * 2.0, 0.0, 1.0);
    out.edges = vertex.edges - h * vertex.shift;
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
