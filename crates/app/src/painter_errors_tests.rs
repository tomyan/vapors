//! A painter's sample count is checked against its format's guaranteed
//! features where the painter is made (`supported_samples`).

use vapors_renderer::{PassFormat, Samples};

use super::{NewPainterError, supported_samples};

fn at(format: wgpu::TextureFormat, samples: Samples) -> PassFormat {
    PassFormat { format, samples }
}

#[test]
fn the_painters_formats_are_accepted_at_4_samples() {
    for format in [
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Bgra8Unorm,
    ] {
        let pass = at(format, Samples::Four);
        assert_eq!(
            supported_samples(pass, wgpu::Features::empty()).ok(),
            Some(pass)
        );
    }
}

#[test]
fn a_format_not_guaranteed_4_samples_is_refused_naming_it() {
    // When
    let pass = at(wgpu::TextureFormat::Rgba32Float, Samples::Four);
    let refused = supported_samples(pass, wgpu::Features::empty());

    // Then
    let Err(error @ NewPainterError::Samples(_)) = refused else {
        panic!("not refused: {refused:?}");
    };
    assert!(
        error
            .to_string()
            .starts_with("ScenePainter at 4 samples on Rgba32Float"),
        "{error}"
    );
}
