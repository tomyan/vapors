// Outer box shadows: a rounded rectangle convolved with a Gaussian of
// standard deviation sigma (half the CSS blur radius). Along x the
// convolution is exact (erf); along y it is integrated with four
// samples, the method Evan Wallace published for GPU shadows
// (madebyevan.com/shaders/fast-rounded-rectangle-shadows). The shadow
// is not drawn under the box that casts it.

struct Instance {
    @location(0) quad: vec4<f32>,
    @location(1) shape: vec4<f32>,
    @location(2) radii: vec4<f32>,
    @location(3) caster: vec4<f32>,
    @location(4) caster_radii: vec4<f32>,
    @location(5) color: vec4<f32>,
    @location(6) extra: vec4<f32>,         // sigma, unused
}

struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) pixel: vec2<f32>,
    @location(1) @interpolate(flat) shape: vec4<f32>,
    @location(2) @interpolate(flat) radii: vec4<f32>,
    @location(3) @interpolate(flat) caster: vec4<f32>,
    @location(4) @interpolate(flat) caster_radii: vec4<f32>,
    @location(5) @interpolate(flat) color: vec4<f32>,
    @location(6) @interpolate(flat) sigma: f32,
}

@group(0) @binding(0) var<uniform> viewport: vec2<f32>;

@vertex
fn vs_main(@builtin(vertex_index) vertex: u32, instance: Instance) -> Out {
    var corners = array<vec2<f32>, 6>(
        vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0),
        vec2(1.0, 0.0), vec2(1.0, 1.0), vec2(0.0, 1.0),
    );
    let pixel = mix(instance.quad.xy, instance.quad.zw, corners[vertex]);
    var out: Out;
    out.position = vec4(pixel.x / viewport.x * 2.0 - 1.0, 1.0 - pixel.y / viewport.y * 2.0, 0.0, 1.0);
    out.pixel = pixel;
    out.shape = instance.shape;
    out.radii = instance.radii;
    out.caster = instance.caster;
    out.caster_radii = instance.caster_radii;
    out.color = instance.color;
    out.sigma = instance.extra.x;
    return out;
}

fn corner_radius(p: vec2<f32>, centre: vec2<f32>, radii: vec4<f32>) -> f32 {
    if (p.x < centre.x) {
        return select(radii.w, radii.x, p.y < centre.y);
    }
    return select(radii.z, radii.y, p.y < centre.y);
}

fn rounded_distance(p: vec2<f32>, rect: vec4<f32>, radii: vec4<f32>) -> f32 {
    let centre = (rect.xy + rect.zw) * 0.5;
    let half = max((rect.zw - rect.xy) * 0.5, vec2(0.0));
    let r = min(corner_radius(p, centre, radii), min(half.x, half.y));
    let q = abs(p - centre) - half + vec2(r);
    return length(max(q, vec2(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

fn gaussian(x: f32, sigma: f32) -> f32 {
    return exp(-(x * x) / (2.0 * sigma * sigma)) / (2.5066283 * sigma);
}

fn erf2(x: vec2<f32>) -> vec2<f32> {
    let s = sign(x);
    let a = abs(x);
    var y = 1.0 + (0.278393 + (0.230389 + 0.078108 * (a * a)) * a) * a;
    y = y * y;
    return s - s / (y * y);
}

fn shadow_x(x: f32, y: f32, sigma: f32, corner: f32, half: vec2<f32>) -> f32 {
    let delta = min(half.y - corner - abs(y), 0.0);
    let curved = half.x - corner + sqrt(max(0.0, corner * corner - delta * delta));
    let integral = 0.5 + 0.5 * erf2((x + vec2(-curved, curved)) * (0.70710678 / sigma));
    return integral.y - integral.x;
}

fn blurred(p: vec2<f32>, rect: vec4<f32>, radii: vec4<f32>, sigma: f32) -> f32 {
    let centre = (rect.xy + rect.zw) * 0.5;
    let half = (rect.zw - rect.xy) * 0.5;
    let corner = min(corner_radius(p, centre, radii), min(half.x, half.y));
    let q = p - centre;
    let low = q.y - half.y;
    let high = q.y + half.y;
    let start = clamp(-3.0 * sigma, low, high);
    let end = clamp(3.0 * sigma, low, high);
    let step = (end - start) / 4.0;
    var y = start + step * 0.5;
    var value = 0.0;
    for (var i = 0; i < 4; i = i + 1) {
        value = value + shadow_x(q.x, q.y - y, sigma, corner, half) * gaussian(y, sigma) * step;
        y = y + step;
    }
    return value;
}

@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    var coverage: f32;
    if (in.sigma < 0.01) {
        coverage = clamp(0.5 - rounded_distance(in.pixel, in.shape, in.radii), 0.0, 1.0);
    } else {
        coverage = blurred(in.pixel, in.shape, in.radii, in.sigma);
    }
    let outside_caster = clamp(0.5 + rounded_distance(in.pixel, in.caster, in.caster_radii), 0.0, 1.0);
    return vec4(in.color.rgb, in.color.a * coverage * outside_caster);
}
