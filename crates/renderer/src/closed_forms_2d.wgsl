// The page's vertex stage for closed-form shapes (the rest is
// closed_forms_shared.wgsl): the instance's affine transform takes shape
// units to device pixels in its frame, and the transform table
// (transform_table.wgsl) places them on the target.

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
fn vs_main(
    @builtin(vertex_index) vertex: u32,
    instance: Instance,
    @location(5) row: u32,   // the row's index in its batch's range
    @location(6) frame: u32, // the frame's entry in the transform table
    @location(7) clip: u32,  // the innermost clip's entry in the clip table
) -> Out {
    let box = local_box(instance);
    let pad = padding(instance.linear);
    let local = mix(box.xy - pad, box.zw + pad, quad_corner(vertex));
    let m = instance.linear;
    let pixel = vec2(m.x * local.x + m.z * local.y, m.y * local.x + m.w * local.y) + instance.shift.xy;
    let position = clip_position(placed(pixel, frame, row));
    var out = vertex_out(instance, local, position);
    out.clip = clip;
    out.row = row;
    return out;
}

@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    let colour = covered(in, dpdx(in.local), dpdy(in.local));
    return vec4(colour.rgb, colour.a * clip_coverage(in.position.xy, in.clip, in.row, view.rows_base));
}
