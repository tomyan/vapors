//! What some demos do when they mount, beyond mounting their component.

use std::time::Duration;

use vapors_elements::Document;

use crate::{PerFrame, stress};

/// The `images` demo's thumbnails, bundled PNGs registered by file
/// name.
///
/// # Panics
/// When a bundled PNG does not decode: a build of this crate is broken.
pub fn register_thumbnails(document: &mut Document) {
    let files: [(&str, &[u8]); 3] = [
        ("sky.png", include_bytes!("../demos/images/sky.png")),
        ("warm.png", include_bytes!("../demos/images/warm.png")),
        ("mono.png", include_bytes!("../demos/images/mono.png")),
    ];
    for (name, bytes) in files {
        let image = vapors_canvas::decode_png(bytes)
            .unwrap_or_else(|e| panic!("crates/demos/demos/images/{name}: {e}"));
        document
            .register_image(name, image)
            .unwrap_or_else(|e| panic!("crates/demos/demos/images/{name}: {e}"));
    }
}

/// The stress demo's tick: the counter moves on by one each frame, and
/// the last frame's time per phase is shown in ms (2-S1). The hook owns
/// the component, which stays mounted while the hook lives, as every
/// demo's does.
pub(crate) fn stress_per_frame(stress: stress::Stress) -> PerFrame {
    Box::new(move |last: &[(&'static str, Duration)]| {
        let component = &stress;
        component.tick.update(|n| *n += 1);
        if !last.is_empty() {
            component.shown.set(phase_line(last));
        }
    })
}

/// `input 0.01 · update 2.80 · draw 0.40 ms`.
pub(crate) fn phase_line(times: &[(&'static str, Duration)]) -> String {
    let parts: Vec<String> = times
        .iter()
        .map(|(name, time)| format!("{name} {:.2}", time.as_secs_f64() * 1000.0))
        .collect();
    format!("{} ms", parts.join(" · "))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::phase_line;

    #[test]
    fn the_phase_line_names_each_phase_with_its_time_in_ms() {
        // Given
        let times = [
            ("input", Duration::from_micros(10)),
            ("update", Duration::from_micros(2_800)),
            ("draw", Duration::from_micros(400)),
        ];

        // Then
        assert_eq!(
            phase_line(&times),
            "input 0.01 · update 2.80 · draw 0.40 ms"
        );
    }
}
