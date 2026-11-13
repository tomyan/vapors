// One textured quad. Positions arrive in device pixels (origin top-left,
// y down), are moved by the scroll frame's shift and go to clip space by
// the viewport; the fragment samples the texture with whatever sampler
// is bound (nearest, for an image canvas).

struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
}

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@group(0) @binding(0) var image: texture_2d<f32>;
@group(0) @binding(1) var image_sampler: sampler;

// The target's size, and the shift of the scroll frame the quad is in.
struct View {
    size: vec2<f32>,
    shift: vec2<f32>,
}

@group(1) @binding(0) var<uniform> view: View;

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    let pixel = in.position + view.shift;
    let clip = vec2<f32>(pixel.x / view.size.x * 2.0 - 1.0, 1.0 - pixel.y / view.size.y * 2.0);
    out.position = vec4<f32>(clip, 0.0, 1.0);
    out.uv = in.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(image, image_sampler, in.uv);
}
