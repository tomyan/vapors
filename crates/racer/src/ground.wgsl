// The racer's ground, a colour for each point on it (`ground_material.rs`):
// grass with stripes and noise, the road, its kerb and verge, the
// shoulder lines, the broken centre marking, the start stripe and grid,
// and the shadows under the building and the trees. `q` is a ground
// point in the simulation's X, Y; `pixel` is how many ground units one
// device pixel spans, so each edge ramps over about a pixel. The track's
// constants, TRACK_STRAIGHT, TRACK_RADIUS and TRACK_ROAD, are put before
// this from `model.rs`.

fn box(q: vec2<f32>, b: vec2<f32>) -> f32 {
    let d = abs(q) - b;
    return length(max(d, vec2(0.))) + min(max(d.x, d.y), 0.);
}
fn mask(d: f32, pixel: f32) -> f32 { return 1. - smoothstep(-pixel, pixel, d); }
fn hash(q: vec2<f32>) -> f32 { return fract(sin(dot(q, vec2(127.1,311.7))) * 43758.5453); }
fn circuit(q: vec2<f32>) -> f32 {
    return length(vec2(q.x - clamp(q.x, -TRACK_STRAIGHT, TRACK_STRAIGHT), q.y)) - TRACK_RADIUS;
}
fn ground(q: vec2<f32>, pixel: f32) -> vec3<f32> {
    let distance = circuit(q);
    let stripe = select(0., 0.025, fract(q.y / 36.) > 0.5);
    var c = vec3(0.23, 0.35, 0.31) + stripe;
    c += (hash(floor(q / 2.)) - 0.5) * 0.018;
    c = mix(c, vec3(0.62,0.63,0.50), mask(abs(distance) - (TRACK_ROAD + 13.),pixel));
    let edge = select(vec3(0.90,0.87,0.75), vec3(0.77,0.29,0.23), sin(atan2(q.y,q.x) * 65.) > 0.);
    c = mix(c,edge,mask(abs(distance) - (TRACK_ROAD + 6.),pixel));
    c = mix(c,vec3(0.18,0.23,0.24),mask(abs(distance) - TRACK_ROAD,pixel));
    c += vec3((hash(floor(q)) - 0.5) * 0.018) * mask(abs(distance)-TRACK_ROAD,pixel);
    let shoulder = abs(abs(distance) - (TRACK_ROAD - 4.)) - 0.7;
    c = mix(c,vec3(0.80,0.80,0.70),mask(shoulder,pixel));
    // Broken centre marking follows the straight/semicircle track.
    let arc = select(q.x, TRACK_STRAIGHT + atan2(q.y, abs(q.x)-TRACK_STRAIGHT)*TRACK_RADIUS, abs(q.x)>TRACK_STRAIGHT);
    if fract(arc / 26.) > 0.65 {
        c = mix(c,vec3(0.58,0.63,0.58),mask(abs(distance)-0.65,pixel));
    }
    // Start stripe, clockwise arrow, and starting-grid boxes.
    if abs(q.x)<7. && abs(q.y+TRACK_RADIUS)<TRACK_ROAD-5. {
        c = select(vec3(0.16,0.20,0.21),vec3(0.94,0.91,0.78), (i32(floor(q.x/7.))+i32(floor(q.y/7.))) % 2 == 0);
    }
    if q.x < -22. && q.x > -125. && abs(q.y+TRACK_RADIUS)<TRACK_ROAD-8. {
        let grid = vec2((q.x+22.) % 28., abs(q.y+TRACK_RADIUS)-15.);
        let line = min(abs(grid.x)-0.5,abs(abs(grid.y)-7.)-0.5);
        if abs(grid.y)<8. { c = mix(c,vec3(0.60,0.65,0.60),mask(line,pixel)); }
    }
    // Ground shadows beneath the solid building and trees.
    c = mix(c,vec3(0.14,0.25,0.23),mask(box(q-vec2(-30.,36.),vec2(100.,36.)),pixel));
    let cell = floor(q/62.); let centre = (cell+0.5)*62.;
    if abs(circuit(centre)) > TRACK_ROAD+37. && length(centre)>250. {
        c = mix(c,vec3(0.16,0.27,0.24),mask(length(q-centre-vec2(5.,7.))-17.,pixel));
    }
    return c;
}
