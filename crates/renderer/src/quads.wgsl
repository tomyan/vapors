// Instanced boxes: each instance is a rectangle in device pixels and a
// non-premultiplied sRGB fill, blended by the pipeline. Six vertices per
// instance make its two triangles.

struct Instance {
    @location(0) rect: vec4<f32>,   // left, top, right, bottom
    @location(1) color: vec4<f32>,
}

struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@group(0) @binding(0) var<uniform> viewport: vec2<f32>;

@vertex
fn vs_main(@builtin(vertex_index) vertex: u32, instance: Instance) -> Out {
    var corners = array<vec2<f32>, 6>(
        vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0),
        vec2(1.0, 0.0), vec2(1.0, 1.0), vec2(0.0, 1.0),
    );
    let corner = corners[vertex];
    let pixel = mix(instance.rect.xy, instance.rect.zw, corner);
    let clip = vec2(pixel.x / viewport.x * 2.0 - 1.0, 1.0 - pixel.y / viewport.y * 2.0);
    var out: Out;
    out.position = vec4(clip, 0.0, 1.0);
    out.color = instance.color;
    return out;
}

@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    return in.color;
}
