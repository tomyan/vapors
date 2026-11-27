use vapors_scene::{Clip, Rect};

fn main() {
    let rect = Rect::new(0.0, 0.0, 4.0, 4.0);
    let _ = Clip {
        radii: [-4.0, 0.0, 0.0, 0.0],
        ..Clip::square(rect)
    };
}
