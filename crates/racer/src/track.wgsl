struct Params {
    viewport: vec4<f32>, // width, height, zoom, time
    camera: vec4<f32>, // camera x/y, car x/y
    car: vec4<f32>, // heading, speed, brake, unused
    track: vec4<f32>, // straight half length, radius, road half width, unused
    view: vec4<f32>, // camera yaw cosine/sine, unused, unused
};
@group(0) @binding(0) var<uniform> p: Params;
@vertex fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let xy = array<vec2<f32>, 3>(vec2(-1., -1.), vec2(3., -1.), vec2(-1., 3.));
    return vec4(xy[index], 0., 1.);
}
fn box(q: vec2<f32>, b: vec2<f32>) -> f32 {
    let d = abs(q) - b;
    return length(max(d, vec2(0.))) + min(max(d.x, d.y), 0.);
}
fn mask(d: f32, pixel: f32) -> f32 { return 1. - smoothstep(-pixel, pixel, d); }
fn hash(q: vec2<f32>) -> f32 { return fract(sin(dot(q, vec2(127.1,311.7))) * 43758.5453); }
fn circuit(q: vec2<f32>) -> f32 {
    return length(vec2(q.x - clamp(q.x, -p.track.x, p.track.x), q.y)) - p.track.y;
}
fn ground(q: vec2<f32>, pixel: f32) -> vec3<f32> {
    let distance = circuit(q);
    let stripe = select(0., 0.025, fract(q.y / 36.) > 0.5);
    var c = vec3(0.23, 0.35, 0.31) + stripe;
    c += (hash(floor(q / 2.)) - 0.5) * 0.018;
    c = mix(c, vec3(0.62,0.63,0.50), mask(abs(distance) - (p.track.z + 13.),pixel));
    let edge = select(vec3(0.90,0.87,0.75), vec3(0.77,0.29,0.23), sin(atan2(q.y,q.x) * 65.) > 0.);
    c = mix(c,edge,mask(abs(distance) - (p.track.z + 6.),pixel));
    c = mix(c,vec3(0.18,0.23,0.24),mask(abs(distance) - p.track.z,pixel));
    c += vec3((hash(floor(q)) - 0.5) * 0.018) * mask(abs(distance)-p.track.z,pixel);
    let shoulder = abs(abs(distance) - (p.track.z - 4.)) - 0.7;
    c = mix(c,vec3(0.80,0.80,0.70),mask(shoulder,pixel));
    // Broken centre marking follows the straight/semicircle track.
    let arc = select(q.x, p.track.x + atan2(q.y, abs(q.x)-p.track.x)*p.track.y, abs(q.x)>p.track.x);
    if fract(arc / 26.) > 0.65 {
        c = mix(c,vec3(0.58,0.63,0.58),mask(abs(distance)-0.65,pixel));
    }
    // Start stripe, clockwise arrow, and starting-grid boxes.
    if abs(q.x)<7. && abs(q.y+p.track.y)<p.track.z-5. {
        c = select(vec3(0.16,0.20,0.21),vec3(0.94,0.91,0.78), (i32(floor(q.x/7.))+i32(floor(q.y/7.))) % 2 == 0);
    }
    if q.x < -22. && q.x > -125. && abs(q.y+p.track.y)<p.track.z-8. {
        let grid = vec2((q.x+22.) % 28., abs(q.y+p.track.y)-15.);
        let line = min(abs(grid.x)-0.5,abs(abs(grid.y)-7.)-0.5);
        if abs(grid.y)<8. { c = mix(c,vec3(0.60,0.65,0.60),mask(line,pixel)); }
    }
    // Ground shadows beneath the solid building and trees.
    c = mix(c,vec3(0.14,0.25,0.23),mask(box(q-vec2(-30.,36.),vec2(100.,36.)),pixel));
    let cell = floor(q/62.); let centre = (cell+0.5)*62.;
    if abs(circuit(centre)) > p.track.z+37. && length(centre)>250. {
        c = mix(c,vec3(0.16,0.27,0.24),mask(length(q-centre-vec2(5.,7.))-17.,pixel));
    }
    return c;
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
