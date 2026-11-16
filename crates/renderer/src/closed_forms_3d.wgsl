// The scene's vertex stage for closed-form shapes (the rest is
// closed_forms_shared.wgsl; docs/plans/decal.md section 2): the
// instance's affine transform takes shape units to a node's plane, z = 0,
// and the node's world matrix and the camera's view-projection take the
// plane to clip space. The fragment stage is the page's: it reads the
// distance's rate across the screen from the derivatives of the
// shape-unit position, which under a projection is interpolated
// perspective-correct, so they are the local Jacobian.

// The camera's view-projection and the target's size in device pixels.
struct Camera {
    view_projection: mat4x4<f32>,
    size: vec2<f32>,
}

// A node's slot: its world matrix; the colour is a mesh's, unread here.
struct Node {
    world: mat4x4<f32>,
    colour: vec4<f32>,
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<uniform> node: Node;

// A clip w at or under this is at or behind the camera: the corner gets
// no padding, and the rasteriser clips the triangle where it crosses.
const NEAR_W: f32 = 1e-6;

// The plane point of shape-unit `local` under the instance's affine.
fn on_plane(instance: Instance, local: vec2<f32>) -> vec2<f32> {
    let m = instance.linear;
    return vec2(m.x * local.x + m.z * local.y, m.y * local.x + m.w * local.y) + instance.shift.xy;
}

// The padding in shape units along x and y that moves each edge of the
// box MARGIN device pixels out at `local`, as the page's `padding` does
// with the screen Jacobian there in place of the affine's: the columns
// are the screen motion per shape unit along x and along y. Lines stay
// lines on a plane, so an edge padded at both its corners stays at
// least the smaller margin from the shape.
fn projective_padding(instance: Instance, to_clip: mat4x4<f32>, local: vec2<f32>) -> vec2<f32> {
    let clip = to_clip * vec4(on_plane(instance, local), 0.0, 1.0);
    if (clip.w <= NEAR_W) {
        return vec2(0.0);
    }
    let to_screen = camera.size * vec2(0.5, -0.5) / (clip.w * clip.w);
    let per_x = to_screen * (to_clip[0].xy * clip.w - clip.xy * to_clip[0].w);
    let per_y = to_screen * (to_clip[1].xy * clip.w - clip.xy * to_clip[1].w);
    let m = instance.linear;
    let along_x = per_x * m.x + per_y * m.y;
    let along_y = per_x * m.z + per_y * m.w;
    let det = abs(along_x.x * along_y.y - along_x.y * along_y.x);
    if (det <= 1e-12) {
        return vec2(0.0);
    }
    return MARGIN * vec2(length(along_y), length(along_x)) / det;
}

@vertex
fn vs_main(@builtin(vertex_index) vertex: u32, instance: Instance) -> Out {
    let box = local_box(instance);
    let at = quad_corner(vertex);
    let corner = mix(box.xy, box.zw, at);
    let to_clip = camera.view_projection * node.world;
    let pad = projective_padding(instance, to_clip, corner);
    let local = corner + (at * 2.0 - 1.0) * pad;
    let position = to_clip * vec4(on_plane(instance, local), 0.0, 1.0);
    return vertex_out(instance, local, position);
}
