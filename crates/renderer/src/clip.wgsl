// The clip table (clip_table.rs, docs/plans/flat-scene-f3.md): each
// instance names the innermost clip it is drawn under, each clip its
// frame's entry in the transform table and the next clip outward.
// Concatenated after transform_table.wgsl and the prelude that sets
// MAX_CLIP_CHAIN (clip_table.rs `clip_prelude`), ahead of each instanced
// pipeline's shader.
//
// A clip's coverage takes the one-pixel ramp a box's edge takes
// (quads.wgsl), not fwidth: before F4a every frame is a whole
// translation (F1 panics on any other class), so a device pixel is a
// pixel of every frame, and the walk below, a loop on the instance's
// clip index, holds no derivative, which WGSL allows only in uniform
// control flow (flat-scene-f3.md item 7).

struct Clip {
    rect: vec4<f32>,  // left, top, right, bottom, device pixels in its frame
    radii: vec4<f32>, // top-left, top-right, bottom-right, bottom-left
    frame: u32,       // its frame's entry in the transform table
    parent: u32,      // the next clip outward; 0 for none
    flags: u32,       // bit 0: relative to the instance's row as well;
                      // bit 1: given back, covering nothing
    pad: u32,
}

@group(0) @binding(2) var<storage, read> clips: array<Clip>;

// The distance past one corner's arc, for a point in that corner's
// square (the r x r square at the corner); otherwise none.
fn corner_term(p: vec2<f32>, corner: vec2<f32>, inward: vec2<f32>, r: f32) -> f32 {
    let centre = corner + inward * r;
    let in_square = all((p - centre) * inward < vec2(0.0));
    return select(-1e9, length(p - centre) - r, r > 0.0 && in_square);
}

// Signed distance from `p` to a rectangle with circular corners of the
// given radii, negative inside. Radii arrive already fitted to the box
// (CSS's single factor, applied on the CPU), so a corner may be larger
// than half a side when its neighbours are small; each corner is cut
// in its own square, so no radius is clamped here.
fn rounded_distance(p: vec2<f32>, rect: vec4<f32>, radii: vec4<f32>) -> f32 {
    let q = max(rect.xy - p, p - rect.zw);
    var d = max(q.x, q.y);
    d = max(d, corner_term(p, rect.xy, vec2(1.0, 1.0), radii.x));
    d = max(d, corner_term(p, vec2(rect.z, rect.y), vec2(-1.0, 1.0), radii.y));
    d = max(d, corner_term(p, rect.zw, vec2(-1.0, -1.0), radii.z));
    d = max(d, corner_term(p, vec2(rect.x, rect.w), vec2(1.0, -1.0), radii.w));
    return d;
}

// How much of the device pixel at `on_target` is inside clip `first`
// and the clips outward from it; 1 for clip 0, which reads no entry.
fn clip_coverage(on_target: vec2<f32>, first: u32, row: u32) -> f32 {
    var coverage = 1.0;
    var index = first;
    for (var link = 0u; link < MAX_CLIP_CHAIN; link++) {
        if (index == 0u) {
            break;
        }
        let clip = clips[index];
        if ((clip.flags & 2u) != 0u) {
            return 0.0;
        }
        var m = transforms[clip.frame];
        if ((clip.flags & 1u) != 0u) {
            m = m * transforms[view.rows_base + row];
        }
        // a whole translation: its inverse is the subtraction
        let p = on_target - m[3].xy;
        coverage = coverage * clamp(0.5 - rounded_distance(p, clip.rect, clip.radii), 0.0, 1.0);
        index = clip.parent;
    }
    return coverage;
}
