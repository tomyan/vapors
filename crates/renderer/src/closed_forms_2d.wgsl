// The page's vertex stage for closed-form shapes (the rest is
// closed_forms_shared.wgsl): the instance's affine transform takes shape
// units to device pixels on the target.

// The target's size, and the shift of the scroll frame the instances
// are in, added to their positions (device pixels, whole).
struct View {
    size: vec2<f32>,
    shift: vec2<f32>,
}

@group(0) @binding(0) var<uniform> view: View;

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
    let box = local_box(instance);
    let pad = padding(instance.linear);
    let local = mix(box.xy - pad, box.zw + pad, quad_corner(vertex));
    let m = instance.linear;
    let pixel = vec2(m.x * local.x + m.z * local.y, m.y * local.x + m.w * local.y) + instance.shift.xy;
    let on_target = pixel + view.shift;
    let position = vec4(on_target.x / view.size.x * 2.0 - 1.0, 1.0 - on_target.y / view.size.y * 2.0, 0.0, 1.0);
    return vertex_out(instance, local, position);
}

@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    return covered(in, dpdx(in.local), dpdy(in.local));
}
