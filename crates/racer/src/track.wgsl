struct Params {
    viewport: vec4<f32>, // width, height, zoom, time
    camera: vec4<f32>, // camera x/y, car x/y
    car: vec4<f32>, // heading, speed, brake, unused
    track: vec4<f32>, // x, y, z written 0 and unread (ground.wgsl's constants hold the track), tilt cosine
    view: vec4<f32>, // camera yaw cosine/sine, unused, unused
};
@group(0) @binding(0) var<uniform> p: Params;
@vertex fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let xy = array<vec2<f32>, 3>(vec2(-1., -1.), vec2(3., -1.), vec2(-1., 3.));
    return vec4(xy[index], 0., 1.);
}
@fragment fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let scale = min(p.viewport.x,p.viewport.y)/p.car.w * p.viewport.z;
    let screen = (position.xy-p.viewport.xy*0.5)/scale;
    let q = view_to_world(vec2(screen.x,screen.y/p.track.w))+p.camera.xy;
    let pixel = 0.8/scale;
    var c = ground(q,pixel);
    let uv = position.xy / p.viewport.xy;
    c *= 1. - 0.13 * dot(uv-0.5,uv-0.5);
    return vec4(c,1.);
}

struct SolidVertex {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};
@vertex fn vs_solid(@location(0) point: vec3<f32>, @location(1) color: vec3<f32>, @location(2) car: f32) -> SolidVertex {
    var world = point;
    if car > 0.5 {
        let cs = cos(p.car.x); let sn = sin(p.car.x);
        world = vec3(point.x*cs-point.y*sn+p.camera.z,point.x*sn+point.y*cs+p.camera.w,point.z);
    }
    let delta = world_to_view(world.xy-p.camera.xy);
    let cosine = p.track.w;
    let sine = sqrt(1.-cosine*cosine);
    let scale = min(p.viewport.x,p.viewport.y)/p.car.w*p.viewport.z;
    let screen = vec2(delta.x,delta.y*cosine-world.z*sine)*scale;
    var output: SolidVertex;
    output.position = vec4(screen.x*2./p.viewport.x,-screen.y*2./p.viewport.y,0.5-(delta.y*sine+world.z*cosine)/4000.,1.);
    output.color = color;
    return output;
}
@fragment fn fs_solid(input: SolidVertex) -> @location(0) vec4<f32> {
    return vec4(input.color,1.);
}

fn world_to_view(delta: vec2<f32>) -> vec2<f32> {
    return vec2(delta.x*p.view.x+delta.y*p.view.y,-delta.x*p.view.y+delta.y*p.view.x);
}
fn view_to_world(delta: vec2<f32>) -> vec2<f32> {
    return vec2(delta.x*p.view.x-delta.y*p.view.y,delta.x*p.view.y+delta.y*p.view.x);
}
