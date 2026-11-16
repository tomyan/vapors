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

// The device pixels (+Y down) moved per shape unit at `local`, as a
// linear map laid out as the page's affine (a, b, c, d: x's column, then
// y's); zero at or behind the camera.
fn screen_jacobian(linear: vec4<f32>, shift: vec2<f32>, local: vec2<f32>) -> vec4<f32> {
    let m = to_clip();
    let clip = m * vec4(on_plane(linear, shift, local), 0.0, 1.0);
    if (clip.w <= NEAR_W) {
        return vec4(0.0);
    }
    let to_screen = camera.size * vec2(0.5, -0.5) / (clip.w * clip.w);
    let per_x = to_screen * (m[0].xy * clip.w - clip.xy * m[0].w);
    let per_y = to_screen * (m[1].xy * clip.w - clip.xy * m[1].w);
    let along_x = per_x * linear.x + per_y * linear.y;
    let along_y = per_x * linear.z + per_y * linear.w;
    return vec4(along_x, along_y);
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
