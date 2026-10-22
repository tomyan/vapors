// One flat-coloured quad. The colour is a uniform: non-premultiplied
// sRGB with alpha, blended by the pipeline.

struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
}

@group(0) @binding(0) var<uniform> color: vec4<f32>;

@vertex
fn vs_main(in: VertexIn) -> @builtin(position) vec4<f32> {
    return vec4<f32>(in.position, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return color;
}
