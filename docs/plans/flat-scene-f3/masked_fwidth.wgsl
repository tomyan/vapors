struct Clip { rect: vec4<f32>, radii: vec4<f32>, frame: u32, parent: u32, flags: u32, pad: u32 }
@group(0) @binding(2) var<storage, read> clips: array<Clip>;

fn distance(p: vec2<f32>, c: Clip) -> f32 {
    let half = (c.rect.zw - c.rect.xy) * 0.5;
    let q = abs(p - (c.rect.xy + half)) - half;
    return length(max(q, vec2(0.0))) + min(max(q.x, q.y), 0.0);
}

const MAX_CLIP_CHAIN: u32 = 4u;

@fragment
fn main(@builtin(position) at: vec4<f32>, @location(0) @interpolate(flat) first: u32) -> @location(0) vec4<f32> {
    // the pixel footprint once, in uniform control flow: before F4a every
    // frame is a whole translation, so a link's distance changes as the
    // device position does
    let footprint = fwidth(at.x) ;
    var coverage = 1.0;
    var index = first;
    for (var link = 0u; link < MAX_CLIP_CHAIN; link++) {
        if index == 0u { break; }
        let c = clips[index];
        let d = distance(at.xy, c);
        coverage = coverage * clamp(0.5 - d / footprint, 0.0, 1.0);
        index = c.parent;
    }
    return vec4(coverage);
}
