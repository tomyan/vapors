// Boxes: each instance is a rectangle in device pixels with a
// background, solid borders (a width and colour per side) and circular
// corner radii. Coverage comes from the signed distance to the rounded
// outer edge, so an edge on a whole pixel is exact and a curve is
// anti-aliased over one pixel. The background fills the whole box under
// the border (background-clip: border-box). Output is non-premultiplied
// sRGB, blended by the pipeline.

struct Instance {
    @location(0) rect: vec4<f32>,          // left, top, right, bottom
    @location(1) background: vec4<f32>,
    @location(2) border: vec4<f32>,        // top, right, bottom, left
    @location(3) top_color: vec4<f32>,
    @location(4) right_color: vec4<f32>,
    @location(5) bottom_color: vec4<f32>,
    @location(6) left_color: vec4<f32>,
    @location(7) radii: vec4<f32>,         // top-left, top-right, bottom-right, bottom-left
    @location(8) row: u32,                 // the row's index in its batch's range
    @location(9) frame: u32,               // the frame's entry in the transform table
    @location(10) clip: u32,               // the innermost clip's entry in the clip table
}

struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) pixel: vec2<f32>,
    @location(1) @interpolate(flat) rect: vec4<f32>,
    @location(2) @interpolate(flat) background: vec4<f32>,
    @location(3) @interpolate(flat) border: vec4<f32>,
    @location(4) @interpolate(flat) top_color: vec4<f32>,
    @location(5) @interpolate(flat) right_color: vec4<f32>,
    @location(6) @interpolate(flat) bottom_color: vec4<f32>,
    @location(7) @interpolate(flat) left_color: vec4<f32>,
    @location(8) @interpolate(flat) radii: vec4<f32>,
    @location(9) @interpolate(flat) clip: u32,
    @location(10) @interpolate(flat) row: u32,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex: u32, instance: Instance) -> Out {
    var corners = array<vec2<f32>, 6>(
        vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0),
        vec2(1.0, 0.0), vec2(1.0, 1.0), vec2(0.0, 1.0),
    );
    let pixel = mix(instance.rect.xy, instance.rect.zw, corners[vertex]);
    var out: Out;
    out.position = clip_position(placed(pixel, instance.frame, instance.row));
    out.pixel = pixel;
    out.rect = instance.rect;
    out.background = instance.background;
    out.border = instance.border;
    out.top_color = instance.top_color;
    out.right_color = instance.right_color;
    out.bottom_color = instance.bottom_color;
    out.left_color = instance.left_color;
    out.radii = instance.radii;
    out.clip = instance.clip;
    out.row = instance.row;
    return out;
}

// The side whose border `p` is in: the edge it is nearest to, measured
// in units of that side's width, which splits each corner on the line
// from the outer to the inner corner, as CSS does.
fn border_color(in: Out) -> vec4<f32> {
    let p = in.pixel;
    let w = max(in.border, vec4(0.0001));
    let top = (p.y - in.rect.y) / w.x;
    let right = (in.rect.z - p.x) / w.y;
    let bottom = (in.rect.w - p.y) / w.z;
    let left = (p.x - in.rect.x) / w.w;
    var best = in.top_color;
    var nearest = select(1e9, top, in.border.x > 0.0);
    if (in.border.y > 0.0 && right < nearest) { nearest = right; best = in.right_color; }
    if (in.border.z > 0.0 && bottom < nearest) { nearest = bottom; best = in.bottom_color; }
    if (in.border.w > 0.0 && left < nearest) { best = in.left_color; }
    return best;
}

@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    let outer = clamp(0.5 - rounded_distance(in.pixel, in.rect, in.radii), 0.0, 1.0);
    let inner_rect = in.rect + vec4(in.border.w, in.border.x, -in.border.y, -in.border.z);
    let widest = max(max(in.border.x, in.border.y), max(in.border.z, in.border.w));
    // Each inner corner is its outer radius less the borders that meet
    // there; the element tree refuses a rounded corner whose two widths
    // differ (that inner corner would be an ellipse), so they are equal.
    let meeting = vec4(
        max(in.border.x, in.border.w),
        max(in.border.x, in.border.y),
        max(in.border.z, in.border.y),
        max(in.border.z, in.border.w),
    );
    let inner_radii = max(in.radii - meeting, vec4(0.0));
    let in_border = clamp(0.5 + rounded_distance(in.pixel, inner_rect, inner_radii), 0.0, 1.0);
    let edge = select(vec4(0.0), border_color(in), widest > 0.0);
    let m = select(0.0, in_border, widest > 0.0);
    // Mix premultiplied, so a border over a transparent background has
    // no dark fringe.
    let a = mix(in.background.a, edge.a, m);
    let rgb = mix(in.background.rgb * in.background.a, edge.rgb * edge.a, m) / max(a, 0.0001);
    return vec4(rgb, a * outer * clip_coverage(in.position.xy, in.clip, in.row));
}
