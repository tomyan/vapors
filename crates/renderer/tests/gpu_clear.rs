//! GPU tier: the clear pass fills every pixel with the requested colour.

use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::clear;
use vapors_scene::Color;

#[test]
fn clear_fills_every_pixel_with_the_colour() {
    // Given
    let gpu = gpu();
    let color = Color::from_rgba8(12, 200, 34, 255);

    // When
    let texture = clear(&gpu.device, &gpu.queue, 7, 5, color);
    let pixels = read_rgba8(&gpu.device, &gpu.queue, &texture);

    // Then
    assert_eq!(pixels.len(), 7 * 5 * 4);
    let (rgba, remainder) = pixels.as_chunks::<4>();
    assert_eq!(remainder, [] as [u8; 0]);
    for pixel in rgba {
        assert_eq!(
            *pixel,
            [12, 200, 34, 255],
            "on {}",
            gpu.adapter.get_info().name
        );
    }
}
