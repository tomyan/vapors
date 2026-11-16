// Closed-form shapes, the part shared by the page's vertex stage
// (closed_forms_2d.wgsl) and the scene's (closed_forms_3d.wgsl): a rect
// with round corners or an ellipse, in its own units, under a transform
// to device pixels (an affine on the page; a plane's projection in the
// scene); filled, or stroked centred on the outline. Coverage is 0.5
// minus the signed distance to the outline in device pixels, clamped:
// 0.5 on the outline, the ramp half a pixel either side of it, as
// Chromium draws an edge. Each quad is the shape's box grown by one
// device pixel each way, so the outer half of the ramp has fragments.
// Output is non-premultiplied sRGB, blended by the pipeline.
//
// The distance is taken in the shape's units and divided by its rate
// of change across the screen: its gradient there, worked out in the
// shape's units (a unit vector), carried to the screen by the map from
// device pixels to shape units at the fragment (`covered`): on the page
// the derivatives of the shape-unit position, exact under its affine;
// in the scene the analytic Jacobian (closed_forms_3d.wgsl), since a
// derivative under a projection is the Jacobian half a pixel or more
// away. The distance's own derivatives would not do:
// it has a kink where its nearest edge changes (a rect's diagonals and
// centre line, a circle's centre), and a 2x2 quad across the kink would
// read it as flat.

struct Instance {
    @location(0) linear: vec4<f32>,  // a, b, c, d of matrix(a, b, c, d, e, f)
    @location(1) shift: vec4<f32>,   // e, f, kind (0 rect, 1 ellipse), stroke width
    @location(2) form: vec4<f32>,    // rect: origin, size; ellipse: centre, radii
    @location(3) radius: vec4<f32>,  // the rect's corner rx, stroked (1) or filled (0), its ry
    @location(4) colour: vec4<f32>,
}

struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    // kind, stroke width, stroked
    @location(1) @interpolate(flat) params: vec3<f32>,
    @location(2) @interpolate(flat) form: vec4<f32>,
    @location(3) @interpolate(flat) colour: vec4<f32>,
    // the rect's corner radii, rx and ry
    @location(4) @interpolate(flat) radii: vec2<f32>,
}

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

// The corner of the instance's quad a vertex is, in 0..1 each way: two
// triangles.
fn quad_corner(vertex: u32) -> vec2<f32> {
    var corners = array<vec2<f32>, 6>(
        vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0),
        vec2(1.0, 0.0), vec2(1.0, 1.0), vec2(0.0, 1.0),
    );
    return corners[vertex];
}

// What the fragment stage reads of a vertex at `local` (shape units)
// placed at `position`.
fn vertex_out(instance: Instance, local: vec2<f32>, position: vec4<f32>) -> Out {
    var out: Out;
    out.position = position;
    out.local = local;
    out.params = vec3(instance.shift.z, instance.shift.w, instance.radius.y);
    out.radii = instance.radius.xz;
    out.form = instance.form;
    out.colour = instance.colour;
    return out;
}

// The distance from `p` to a rect at `origin` of `size` with corners
// of `radii` (within half each side, both zero or both above: the
// shapes crate's check refuses the rest), negative inside, and its
// gradient's direction.
fn rect_distance(p: vec2<f32>, origin: vec2<f32>, size: vec2<f32>, radii: vec2<f32>) -> vec3<f32> {
    let half = size / 2.0;
    let from_centre = p - origin - half;
    let side = select(vec2(-1.0), vec2(1.0), from_centre >= vec2(0.0));
    let folded = abs(from_centre);
    if (radii.x == radii.y) {
        return round_corners(folded, half, radii.x, side);
    }
    return elliptical_corners(folded, half, radii, side);
}

// The distance from `folded` (the position from the rect's centre, in
// the corner's quadrant) to a rect of `half` its size with circular
// corners of `radius`, exact, and its gradient back in the `side`
// quadrant: a unit vector.
fn round_corners(folded: vec2<f32>, half: vec2<f32>, radius: f32, side: vec2<f32>) -> vec3<f32> {
    let q = folded - half + radius;
    let outer = max(q, vec2(0.0));
    let in_corner = q.x > 0.0 && q.y > 0.0;
    let d = length(outer) + min(max(q.x, q.y), 0.0) - radius;
    let nearer_x = q.x > q.y;
    let axis = select(vec2(0.0, side.y), vec2(side.x, 0.0), nearer_x);
    let gradient = select(axis, side * normalize(outer), in_corner);
    return vec3(d, gradient);
}

// As `round_corners`, with elliptical corners of `radii`, exact to the
// refinement's convergence at any depth: the nearest of the top edge,
// the side edge and the quarter ellipse, each as a piece of the
// outline (a point past an edge's end is nearer the arc). The sign
// is taken from the outline itself, so a stroke deeper than the
// smaller radius stays continuous where the pieces meet.
fn elliptical_corners(folded: vec2<f32>, half: vec2<f32>, radii: vec2<f32>, side: vec2<f32>) -> vec3<f32> {
    let corner = half - radii;
    let q = folded - corner;
    var d = length(vec2(max(q.x, 0.0), folded.y - half.y));
    var normal = vec2(0.0, 1.0);
    let to_side = length(vec2(folded.x - half.x, max(q.y, 0.0)));
    if (to_side < d) {
        d = to_side;
        normal = vec2(1.0, 0.0);
    }
    if (any(q > vec2(0.0))) {
        let t = quarter_ellipse(q, radii);
        let to_arc = length(q - radii * t);
        if (to_arc < d) {
            d = to_arc;
            normal = normalize(vec2(radii.y * t.x, radii.x * t.y));
        }
    }
    let past_arc = all(q > vec2(0.0)) && dot(q / radii, q / radii) > 1.0;
    let inside = all(folded < half) && !past_arc;
    return vec3(select(d, -d, inside), side * normal);
}

// Refinement steps for `quarter_ellipse`. Measured against the exact
// distance (a dense search refined by ternary search) over radii from
// 1 x 1 to 200 x 25 px, ratios to 8:1, points across the whole corner:
// 3 steps leave 0.0077 px at 100 x 50 px, 4 leave 3e-5 px, so 4.
const ELLIPSE_STEPS: i32 = 4;

// The point of the quarter ellipse of `radii` (x, y >= 0, centred on
// the origin) nearest `q`, as the (cos t, sin t) of (rx cos t, ry sin t).
// Each step moves along the ellipse by the arc of the circle of
// curvature through the current point, centred on the evolute (C.
// Chatfield's trig-free method), clamped to the quarter. A point the
// clamp leaves with no direction (q below and left of the centre)
// takes the end of the nearer axis.
fn quarter_ellipse(q: vec2<f32>, radii: vec2<f32>) -> vec2<f32> {
    let k = (radii.x * radii.x - radii.y * radii.y) * vec2(1.0 / radii.x, -1.0 / radii.y);
    var t = vec2(0.70710678);
    for (var step = 0; step < ELLIPSE_STEPS; step++) {
        let evolute = k * t * t * t;
        let radius = length(radii * t - evolute);
        let towards = q - evolute;
        let along = towards * radius / max(length(towards), 1e-30);
        let next = clamp((along + evolute) / radii, vec2(0.0), vec2(1.0));
        let axis = select(vec2(0.0, 1.0), vec2(1.0, 0.0), q.x > q.y);
        t = select(normalize(next), axis, all(next == vec2(0.0)));
    }
    return t;
}

// The distance from `p` to an ellipse about `centre` with `radii`,
// negative inside, exact to the refinement's convergence, and its
// gradient: the outward normal at the nearest point, a unit vector. A
// circle takes its exact distance without the refinement; the radii
// are flat per instance, so the branch is the same across a quad.
fn ellipse_distance(p: vec2<f32>, centre: vec2<f32>, radii: vec2<f32>) -> vec3<f32> {
    let q = p - centre;
    if (radii.x == radii.y) {
        let gradient = select(vec2(1.0, 0.0), normalize(q), dot(q, q) > 0.0);
        return vec3(length(q) - radii.x, gradient);
    }
    let side = select(vec2(-1.0), vec2(1.0), q >= vec2(0.0));
    let folded = abs(q);
    let t = quarter_ellipse(folded, radii);
    let d = length(folded - radii * t);
    let inside = dot(folded / radii, folded / radii) < 1.0;
    let normal = normalize(vec2(radii.y * t.x, radii.x * t.y));
    return vec3(select(d, -d, inside), side * normal);
}

// The colour of `in` with its coverage, `across_x` and `across_y` the
// shape units the position moves per device pixel right and down there:
// the page takes them from the position's derivatives, exact under its
// affine; the scene from the analytic Jacobian (closed_forms_3d.wgsl).
fn covered(in: Out, across_x: vec2<f32>, across_y: vec2<f32>) -> vec4<f32> {
    var dg = rect_distance(in.local, in.form.xy, in.form.zw, in.radii);
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
