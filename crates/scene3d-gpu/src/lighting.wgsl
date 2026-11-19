// Ambient and Lambert diffuse over the lights uniform (lights.rs holds its
// layout), in linear light. A chunk of its own so the flat scene's mesh
// pipeline can use it later (docs/plans/3d-engine-parity.md, G4).

struct Light {
    // linear colour times intensity, then the kind: 0 directional, 1 point
    colour: vec3<f32>,
    kind: u32,
    position: vec3<f32>,
    range: f32,
    // the way a directional light shines
    direction: vec3<f32>,
    unused: f32,
}

struct Lights {
    ambient: vec3<f32>,
    unused: f32,
    count: u32,
    unused_1: u32,
    unused_2: u32,
    unused_3: u32,
    lights: array<Light, 8>,
}

@group(2) @binding(0) var<uniform> lights: Lights;

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    return select(pow((c + 0.055) / 1.055, vec3(2.4)), c / 12.92, c <= vec3(0.04045));
}

fn linear_to_srgb(c: vec3<f32>) -> vec3<f32> {
    return select(1.055 * pow(c, vec3(1.0 / 2.4)) - 0.055, c * 12.92, c <= vec3(0.0031308));
}

// What one light gives a surface: colour, falloff (1 for a directional
// light; a point light's falls smoothly to 0 at range) and diffuse term.
fn light_gives(light: Light, albedo: vec3<f32>, world: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    var to_light = -light.direction;
    var falloff = 1.0;
    if light.kind == 1u {
        let offset = light.position - world;
        let inside = saturate(1.0 - dot(offset, offset) / (light.range * light.range));
        to_light = offset / max(length(offset), 1e-6);
        falloff = inside * inside;
    }
    let lambert = max(dot(normal, to_light), 0.0);
    return light.colour * falloff * albedo * lambert;
}

// `srgb` lit at `world` (`normal` a unit vector), encoded back to sRGB,
// times the vertex `shade`.
fn lit_colour(srgb: vec3<f32>, shade: f32, world: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    let albedo = srgb_to_linear(srgb);
    var sum = lights.ambient * albedo;
    for (var i = 0u; i < lights.count; i++) {
        sum += light_gives(lights.lights[i], albedo, world, normal);
    }
    return linear_to_srgb(sum) * shade;
}
