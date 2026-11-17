@fragment
fn main(@builtin(position) at: vec4<f32>) -> @location(0) vec4<f32> {
    var w = 0.0;
    if at.x > 10.0 {
        w = fwidth(at.y);
    }
    return vec4(w);
}
