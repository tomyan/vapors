//! What some demos do when they mount, beyond mounting their component.

use std::time::Duration;

use vapors_elements::Document;

use crate::{PerFrame, stress};

/// The `images` demo's thumbnails, bundled PNGs registered by file
/// name.
///
/// # Panics
/// When a bundled PNG does not decode: a build of this crate is broken.
pub(crate) fn register_thumbnails(document: &mut Document) {
    let files: [(&str, &[u8]); 3] = [
        ("sky.png", include_bytes!("../demos/images/sky.png")),
        ("warm.png", include_bytes!("../demos/images/warm.png")),
        ("mono.png", include_bytes!("../demos/images/mono.png")),
    ];
    for (name, bytes) in files {
        let image = vapors_canvas::decode_png(bytes)
            .unwrap_or_else(|e| panic!("crates/demos/demos/images/{name}: {e}"));
        document.register_image(name, image);
    }
}

/// The stress demo's tick: the counter moves on by one each frame, and
/// the last update's time is shown in ms. The hook owns the component,
/// which stays mounted while the hook lives, as every demo's does.
pub(crate) fn stress_per_frame(stress: stress::Stress) -> PerFrame {
    Box::new(move |last: Option<Duration>| {
        let component = &stress;
        component.tick.update(|n| *n += 1);
        if let Some(last) = last {
            component
                .shown
                .set(format!("{:.2} ms", last.as_secs_f64() * 1000.0));
        }
    })
}
