// One textured quad. Positions arrive in device pixels (origin top-left,
// y down) in the quad's frame and row, are placed by their entries in the
// transform table (transform_table.rs) and go to clip space by the
// viewport; the fragment samples the texture with whatever sampler
// is bound (nearest, for an image canvas) and takes the coverage of the
// quad's clip in the clip table (clip.wgsl, concatenated after this:
// flat-scene F3.1b).

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

// The target's size, the quad's frame's entry in the transform table,
// its row's (0, the identity, for a quad in no lane row), and its clip's
// entry in the clip table (0 for none).
struct View {
    size: vec2<f32>,
    frame: u32,
    row: u32,
    clip: u32,
}

@group(1) @binding(0) var<uniform> view: View;

@group(1) @binding(1) var<storage, read> transforms: array<mat4x4<f32>>;

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    let m = transforms[view.frame] * transforms[view.row];
    let pixel = (m * vec4(in.position, 0.0, 1.0)).xy;
    let clip = vec2<f32>(pixel.x / view.size.x * 2.0 - 1.0, 1.0 - pixel.y / view.size.y * 2.0);
    out.position = vec4<f32>(clip, 0.0, 1.0);
    out.uv = in.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let colour = textureSample(image, image_sampler, in.uv);
    // the row is already an entry of its own, so no draw base is added
    return vec4(colour.rgb, colour.a * clip_coverage(in.position.xy, view.clip, view.row, 0u));
}
