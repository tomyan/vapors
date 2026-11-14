// Closed-form shapes: a rect with circular corners or an ellipse, in
// its own units, under an affine transform to device pixels; filled, or
// stroked centred on the outline. Coverage is 0.5 minus the signed
// distance to the outline in device pixels, clamped: 0.5 on the
// outline, the ramp half a pixel either side of it, as Chromium draws
// an edge. Each quad is the shape's box grown by one device pixel each
// way, so the outer half of the ramp has fragments. Output is
// non-premultiplied sRGB, blended by the pipeline.
//
// The distance is taken in the shape's units and divided by its rate
// of change across the screen: its gradient there, worked out in the
// shape's units (a unit vector), carried to the screen by the
// derivatives of the shape-unit position, which is affine in screen
// space and so has exact derivatives. The distance's own derivatives
// would not do: it has a kink where its nearest edge changes (a rect's
// diagonals and centre line, a circle's centre), and a 2x2 quad across
// the kink would read it as flat.

struct Instance {
    @location(0) linear: vec4<f32>,  // a, b, c, d of matrix(a, b, c, d, e, f)
    @location(1) shift: vec4<f32>,   // e, f, kind (0 rect, 1 ellipse), stroke width
    @location(2) form: vec4<f32>,    // rect: origin, size; ellipse: centre, radii
    @location(3) radius: vec4<f32>,  // the rect's corner radius, stroked (1) or filled (0)
    @location(4) colour: vec4<f32>,
}

struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    // kind, stroke width, stroked, the rect's corner radius
    @location(1) @interpolate(flat) params: vec4<f32>,
    @location(2) @interpolate(flat) form: vec4<f32>,
    @location(3) @interpolate(flat) colour: vec4<f32>,
}

// The target's size, and the shift of the scroll frame the instances
// are in, added to their positions (device pixels, whole).
struct View {
    size: vec2<f32>,
    shift: vec2<f32>,
}

@group(0) @binding(0) var<uniform> view: View;

// How far past the shape the quad reaches, device pixels: the ramp's
// outer half needs half; the rest is slack for the rasteriser.
const MARGIN: f32 = 1.0;

// The shape's box in its own units, low and high corners, grown by half
// the stroke.
fn local_box(instance: Instance) -> vec4<f32> {
    let half_stroke = instance.radius.y * instance.shift.w / 2.0;
    var lo = instance.form.xy - instance.form.zw;
    var hi = instance.form.xy + instance.form.zw;
    if (instance.shift.z == 0.0) {
        lo = instance.form.xy;
        hi = instance.form.xy + instance.form.zw;
    }
    return vec4(lo - half_stroke, hi + half_stroke);
}

// The padding in local units along x and y that moves each edge of the
// box MARGIN device pixels out, perpendicular to it, under any affine
// map: an edge x = const runs along column (c, d), and moving it by p
// along x moves it p |det| / |(c, d)| across. The CPU refuses a
// transform with no area before it reaches here.
fn padding(linear: vec4<f32>) -> vec2<f32> {
    let det = abs(linear.x * linear.w - linear.y * linear.z);
    return MARGIN * vec2(length(linear.zw), length(linear.xy)) / det;
}

@vertex
fn vs_main(@builtin(vertex_index) vertex: u32, instance: Instance) -> Out {
    var corners = array<vec2<f32>, 6>(
        vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0),
        vec2(1.0, 0.0), vec2(1.0, 1.0), vec2(0.0, 1.0),
    );
    let box = local_box(instance);
    let pad = padding(instance.linear);
    let local = mix(box.xy - pad, box.zw + pad, corners[vertex]);
    let m = instance.linear;
    let pixel = vec2(m.x * local.x + m.z * local.y, m.y * local.x + m.w * local.y) + instance.shift.xy;
    let on_target = pixel + view.shift;
    var out: Out;
    out.position = vec4(on_target.x / view.size.x * 2.0 - 1.0, 1.0 - on_target.y / view.size.y * 2.0, 0.0, 1.0);
    out.local = local;
    out.params = vec4(instance.shift.z, instance.shift.w, instance.radius.y, instance.radius.x);
    out.form = instance.form;
    out.colour = instance.colour;
    return out;
}

// The distance from `p` to a rect at `origin` of `size` with circular
// corners of `radius` (within half each side: the shapes crate's check
// refuses more), negative inside, and its gradient: a unit vector.
fn rect_distance(p: vec2<f32>, origin: vec2<f32>, size: vec2<f32>, radius: f32) -> vec3<f32> {
    let half = size / 2.0;
    let from_centre = p - origin - half;
    let side = select(vec2(-1.0), vec2(1.0), from_centre >= vec2(0.0));
    let q = abs(from_centre) - half + radius;
    let outer = max(q, vec2(0.0));
    let in_corner = q.x > 0.0 && q.y > 0.0;
    let d = length(outer) + min(max(q.x, q.y), 0.0) - radius;
    let nearer_x = q.x > q.y;
    let axis = select(vec2(0.0, side.y), vec2(side.x, 0.0), nearer_x);
    let gradient = select(axis, side * normalize(outer), in_corner);
    return vec3(d, gradient);
}

// The distance from `p` to an ellipse about `centre` with `radii`,
// negative inside (exact for a circle, to first order at any ellipse's
// outline, where coverage is taken), and its gradient's direction. At
// the centre, where the formula is 0 / 0, the smaller radius.
fn ellipse_distance(p: vec2<f32>, centre: vec2<f32>, radii: vec2<f32>) -> vec3<f32> {
    let q = p - centre;
    let k0 = length(q / radii);
    let k1 = length(q / (radii * radii));
    let d = select(-min(radii.x, radii.y), k0 * (k0 - 1.0) / k1, k1 > 0.0);
    let normal = q / (radii * radii);
    let gradient = select(vec2(1.0, 0.0), normalize(normal), dot(normal, normal) > 0.0);
    return vec3(d, gradient);
}

@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    let across_x = dpdx(in.local);
    let across_y = dpdy(in.local);
    var dg = rect_distance(in.local, in.form.xy, in.form.zw, in.params.w);
    if (in.params.x != 0.0) {
        dg = ellipse_distance(in.local, in.form.xy, in.form.zw);
    }
    // Shape units per device pixel along the gradient.
    let g = dg.yz;
    let per_pixel = max(length(vec2(dot(g, across_x), dot(g, across_y))), 1e-20);
    let stroked = in.params.z != 0.0;
    let outside = select(dg.x, abs(dg.x) - in.params.y / 2.0, stroked);
    let coverage = clamp(0.5 - outside / per_pixel, 0.0, 1.0);
    return vec4(in.colour.rgb, in.colour.a * coverage);
}
