// A horizontal gradient over the whole surface: one triangle covering
// the viewport, coloured by its x position, there and back across two
// widths, moved along by `shift` (0 to 1). The colours are written as
// sRGB-encoded values, the convention of SURFACE_FORMAT.

@group(0) @binding(0) var<uniform> shift: f32;

struct Varyings {
    @builtin(position) position: vec4<f32>,
    @location(0) across: f32,
}

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> Varyings {
    let x = f32((index << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(index & 2u) * 2.0 - 1.0;
    var out: Varyings;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.across = (x + 1.0) * 0.5;
    return out;
}

const LEFT: vec3<f32> = vec3<f32>(0.10, 0.20, 0.60);
const RIGHT: vec3<f32> = vec3<f32>(0.95, 0.55, 0.15);

@fragment
fn fragment(in: Varyings) -> @location(0) vec4<f32> {
    let along = 1.0 - abs(fract(in.across * 0.5 + shift) * 2.0 - 1.0);
    return vec4<f32>(mix(LEFT, RIGHT, along), 1.0);
}
