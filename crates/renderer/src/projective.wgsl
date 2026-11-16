// What every vertex stage drawing shapes on a scene node's plane shares
// (docs/plans/decal.md section 2): the camera and the node's slot, and
// the screen Jacobian of a plane point, from which each stage moves its
// outline out by device pixels.

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

// A clip w at or under this is at or behind the camera: its Jacobian is
// taken as none, so nothing is moved there, and the rasteriser clips the
// triangle where it crosses. Any w under the near plane is clipped
// whatever its padding (a perspective's w is the view depth, and
// `Projection` refuses a near plane at or under 0; an orthographic w is
// 1), so the value only has to keep 1 / w^2 finite in f32: 1e12 here.
const NEAR_W: f32 = 1e-6;

// Shape units to the node's plane, z = 0 there: the affine
// matrix(a, b, c, d, e, f) as `linear` (a, b, c, d) and `shift` (e, f).
fn on_plane(linear: vec4<f32>, shift: vec2<f32>, local: vec2<f32>) -> vec2<f32> {
    return vec2(linear.x * local.x + linear.z * local.y, linear.y * local.x + linear.w * local.y) + shift;
}

// Shape units to clip space.
fn to_clip() -> mat4x4<f32> {
    return camera.view_projection * node.world;
}

// Shape units to clip space as three columns: clip = k[0] x + k[1] y +
// k[2] for shape point (x, y) under the affine (`linear`, `shift`), the
// node's world matrix and the camera. Per instance: a vertex stage works
// it out once and passes it on flat, so a fragment needs no matrix
// product (#424's review, L2).
fn shape_to_clip(linear: vec4<f32>, shift: vec2<f32>) -> mat3x4<f32> {
    let m = to_clip();
    return mat3x4(m * vec4(linear.xy, 0.0, 0.0), m * vec4(linear.zw, 0.0, 0.0), m * vec4(shift, 0.0, 1.0));
}

// The device pixels (+Y down) moved per shape unit at `local` under `k`
// (`shape_to_clip`), as a linear map laid out as the page's affine (a,
// b, c, d: x's column, then y's); zero at or behind the camera.
fn jacobian_at(k: mat3x4<f32>, local: vec2<f32>) -> vec4<f32> {
    let clip = k[0] * local.x + k[1] * local.y + k[2];
    if (clip.w <= NEAR_W) {
        return vec4(0.0);
    }
    let to_screen = camera.size * vec2(0.5, -0.5) / (clip.w * clip.w);
    let along_x = to_screen * (k[0].xy * clip.w - clip.xy * k[0].w);
    let along_y = to_screen * (k[1].xy * clip.w - clip.xy * k[1].w);
    return vec4(along_x, along_y);
}

// `jacobian_at` for an instance's affine, working out its columns.
fn screen_jacobian(linear: vec4<f32>, shift: vec2<f32>, local: vec2<f32>) -> vec4<f32> {
    return jacobian_at(shape_to_clip(linear, shift), local);
}

// Whether a screen Jacobian `j` (as `screen_jacobian` gives it) is
// edge-on: its columns parallel, or zero behind the eye. The test is the
// sine of the angle between the columns, independent of the view box's
// scale: under 1e-6 the plane is within 6e-5 degrees of edge-on, a
// sliver under a millionth of the shape's width on screen. The one
// answer every scene stage asks.
fn edge_on(j: vec4<f32>) -> bool {
    let det = abs(j.x * j.w - j.y * j.z);
    return det <= 1e-6 * length(j.xy) * length(j.zw);
}
