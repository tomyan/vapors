// The lit mesh (docs/plans/lighting.md, LT3): the normal goes through the
// world matrix's cofactor matrix (right under non-uniform scale and
// mirroring). After PROJECTIVE_WGSL and `lighting.wgsl` (lit.rs).

struct LitVertex {
    @builtin(position) position: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) shade: f32,
}

fn normal_matrix(m: mat3x3<f32>) -> mat3x3<f32> {
    let cofactor = mat3x3<f32>(cross(m[1], m[2]), cross(m[2], m[0]), cross(m[0], m[1]));
    return cofactor * sign(determinant(m));
}

@vertex
fn vs_lit(
    @location(0) position: vec3<f32>,
    @location(1) shade: f32,
    @location(2) normal: vec3<f32>,
) -> LitVertex {
    let world = node.world * vec4<f32>(position, 1.0);
    let linear = mat3x3<f32>(node.world[0].xyz, node.world[1].xyz, node.world[2].xyz);
    return LitVertex(camera.view_projection * world, world.xyz, normal_matrix(linear) * normal, shade);
}

@fragment
fn fs_lit(in: LitVertex) -> @location(0) vec4<f32> {
    let rgb = lit_colour(node.colour.rgb, in.shade, in.world, normalize(in.normal));
    return vec4<f32>(rgb, node.colour.a);
}
