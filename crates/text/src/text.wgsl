// Glyph quads from the coverage atlas. Positions arrive in device
// pixels (origin top-left, y down) in their frame and row, are placed by
// their entries in the transform table (vapors-renderer
// transform_table.rs) and go to clip space by the viewport. Each
// vertex names its clip's entry in the clip table, whose coverage the
// fragment takes (vapors-renderer clip.wgsl, concatenated after this:
// flat-scene F3.1b).
// The fragment turns coverage into an alpha the way Skia's mask gamma
// does: blend text and an assumed background in linear light, with a
// contrast boost that fades as the text gets lighter, then find the
// alpha that gives that result when the hardware blends in encoded
// space. Two parameters: gamma (the encoding exponent) and contrast.

// The blend parameters, the viewport, and the first entry of the draw's
// row range in the transform table.
struct Params {
    gamma: f32,
    contrast: f32,
    viewport: vec2<f32>,
    rows_base: u32,
}

struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) row: u32,                 // the row's index in its draw's range
    @location(4) frame: u32,               // the frame's entry in the transform table
    @location(5) clip: u32,                // the innermost clip's entry in the clip table
}

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) clip: u32,
    @location(3) @interpolate(flat) row: u32,
}

@group(0) @binding(0) var atlas: texture_2d<f32>;
@group(0) @binding(1) var atlas_sampler: sampler;
@group(0) @binding(2) var<uniform> params: Params;
@group(0) @binding(3) var<storage, read> transforms: array<mat4x4<f32>>;

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    let m = transforms[in.frame] * transforms[params.rows_base + in.row];
    let pixel = (m * vec4(in.position, 0.0, 1.0)).xy;
    let clip = vec2<f32>(
        pixel.x / params.viewport.x * 2.0 - 1.0,
        1.0 - pixel.y / params.viewport.y * 2.0,
    );
    out.position = vec4<f32>(clip, 0.0, 1.0);
    out.uv = in.uv;
    out.color = in.color;
    out.clip = in.clip;
    out.row = in.row;
    return out;
}

// Encoded luminance of the ink: the channels are linearised, weighted,
// and re-encoded. For grey ink this is the grey value itself.
fn luminance(color: vec3<f32>) -> f32 {
    let linear = pow(color, vec3<f32>(params.gamma));
    let lum = dot(linear, vec3<f32>(0.2126, 0.7152, 0.0722));
    return pow(lum, 1.0 / params.gamma);
}

// Coverage to alpha for text of encoded luminance `src` over the
// assumed background `1 - src`, as Skia's SkMaskGamma builds its table:
// the contrast boost always applies; only the final solve needs src and
// dst apart, and at the midpoint the boosted coverage is the answer.
fn adjust(coverage: f32, src: f32) -> f32 {
    let dst = 1.0 - src;
    let lin_src = pow(src, params.gamma);
    let lin_dst = pow(dst, params.gamma);
    let contrast = params.contrast * lin_dst;
    let boosted = coverage + coverage * (1.0 - coverage) * contrast;
    if abs(src - dst) < 1.0 / 256.0 {
        return boosted;
    }
    let lin = lin_src * boosted + lin_dst * (1.0 - boosted);
    let result = pow(lin, 1.0 / params.gamma);
    return clamp((result - dst) / (src - dst), 0.0, 1.0);
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let coverage = textureSample(atlas, atlas_sampler, in.uv).r;
    let clipped = clip_coverage(in.position.xy, in.clip, in.row, params.rows_base);
    let alpha = adjust(coverage, luminance(in.color.rgb)) * in.color.a * clipped;
    return vec4<f32>(in.color.rgb * alpha, alpha);
}
