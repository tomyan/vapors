// Glyph quads from the coverage atlas. Positions arrive in clip space.
// The fragment turns coverage into an alpha the way Skia's mask gamma
// does: blend text and an assumed background in linear light, with a
// contrast boost that fades as the text gets lighter, then find the
// alpha that gives that result when the hardware blends in encoded
// space. Two parameters: gamma (the encoding exponent) and contrast.

struct Params {
    gamma: f32,
    contrast: f32,
    _pad: vec2<f32>,
}

struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@group(0) @binding(0) var atlas: texture_2d<f32>;
@group(0) @binding(1) var atlas_sampler: sampler;
@group(0) @binding(2) var<uniform> params: Params;

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = vec4<f32>(in.position, 0.0, 1.0);
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

fn luminance(color: vec3<f32>) -> f32 {
    return dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
}

// Coverage to alpha for text of encoded luminance `src` over the
// assumed background `1 - src`.
fn adjust(coverage: f32, src: f32) -> f32 {
    let dst = 1.0 - src;
    if abs(src - dst) < 1.0 / 256.0 {
        return coverage;
    }
    let lin_src = pow(src, params.gamma);
    let lin_dst = pow(dst, params.gamma);
    let contrast = params.contrast * lin_dst;
    let boosted = coverage + coverage * (1.0 - coverage) * contrast;
    let lin = lin_src * boosted + lin_dst * (1.0 - boosted);
    let result = pow(lin, 1.0 / params.gamma);
    return clamp((result - dst) / (src - dst), 0.0, 1.0);
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let coverage = textureSample(atlas, atlas_sampler, in.uv).r;
    let alpha = adjust(coverage, luminance(in.color.rgb)) * in.color.a;
    return vec4<f32>(in.color.rgb * alpha, alpha);
}
