// MSAA: each triangle filled solid, positions already in clip space;
// the hardware's samples give the coverage.
struct Vertex {
    @location(0) position: vec2<f32>,
    @location(1) colour: vec4<f32>,
}

struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) colour: vec4<f32>,
}

@vertex
fn vs_main(v: Vertex) -> Out {
    var out: Out;
    out.position = vec4(v.position, 0.0, 1.0);
    out.colour = v.colour;
    return out;
}

@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    return in.colour;
}
