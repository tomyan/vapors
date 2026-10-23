//! GPU tier: borders take their own side's colour and width, the
//! background fills under them, and a rounded corner leaves its outer
//! pixel to what is behind.

use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::{QuadPipeline, ShadowPipeline, Target, clear};
use vapors_scene::{Color, Quad, Rect, Shadow, Size};

const BACK: [u8; 4] = [40, 40, 40, 255];

fn colour(rgba: [u8; 4]) -> Color {
    Color::from_rgba8(rgba[0], rgba[1], rgba[2], rgba[3])
}

fn pixel(pixels: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let start = ((y * width + x) * 4) as usize;
    pixels[start..start + 4].try_into().expect("four bytes")
}

/// `draw` run on a fresh `width` x `height` target cleared to BACK.
fn render(width: u32, height: u32, draw: impl FnOnce(&wgpu::Device, &Target<'_>)) -> Vec<u8> {
    let gpu = gpu();
    let target = clear(&gpu.device, &gpu.queue, width, height, colour(BACK));
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let viewport = Size { width, height };
    draw(
        &gpu.device,
        &Target {
            device: &gpu.device,
            queue: &gpu.queue,
            view: &view,
            viewport,
        },
    );
    read_rgba8(&gpu.device, &gpu.queue, &target)
}

fn boxes(quads: &[Quad], width: u32, height: u32) -> Vec<u8> {
    render(width, height, |device, target| {
        QuadPipeline::new(device, wgpu::TextureFormat::Rgba8Unorm).draw(target, quads);
    })
}

#[test]
fn gpu_each_side_of_a_border_has_its_own_width_and_colour_over_the_background() {
    // Given: a 10x10 box, borders top 1 red, right 2 green, bottom 3 blue, left 4 white
    let (red, green, blue, white, fill) = (
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 255, 255],
        [9, 9, 9, 255],
    );
    let quad = Quad {
        border: [1.0, 2.0, 3.0, 4.0],
        border_colors: [colour(red), colour(green), colour(blue), colour(white)],
        ..Quad::fill(Rect::new(0.0, 0.0, 10.0, 10.0), colour(fill))
    };

    // When
    let pixels = boxes(&[quad], 12, 12);

    // Then
    assert_eq!(pixel(&pixels, 12, 5, 0), red, "top, 1 px");
    assert_eq!(
        pixel(&pixels, 12, 5, 1),
        fill,
        "inside, below the top border"
    );
    assert_eq!(pixel(&pixels, 12, 8, 5), green, "right, 2 px");
    assert_eq!(
        pixel(&pixels, 12, 7, 5),
        fill,
        "inside, left of the right border"
    );
    assert_eq!(pixel(&pixels, 12, 5, 7), blue, "bottom, 3 px");
    assert_eq!(pixel(&pixels, 12, 3, 5), white, "left, 4 px");
    assert_eq!(
        pixel(&pixels, 12, 4, 5),
        fill,
        "inside, right of the left border"
    );
    assert_eq!(pixel(&pixels, 12, 10, 5), BACK, "outside the box");
}

#[test]
fn gpu_a_rounded_corner_leaves_its_corner_pixel_to_the_background() {
    // Given: a 20x20 box with 8 px radii
    let quad = Quad {
        radii: [8.0; 4],
        ..Quad::fill(
            Rect::new(0.0, 0.0, 20.0, 20.0),
            colour([255, 255, 255, 255]),
        )
    };

    // When
    let pixels = boxes(&[quad], 20, 20);

    // Then
    assert_eq!(
        pixel(&pixels, 20, 0, 0),
        BACK,
        "the corner is outside the curve"
    );
    assert_eq!(
        pixel(&pixels, 20, 10, 0),
        [255, 255, 255, 255],
        "the straight top edge is filled"
    );
    assert_eq!(pixel(&pixels, 20, 10, 10), [255, 255, 255, 255]);
    let edge = pixel(&pixels, 20, 2, 2)[0];
    assert!(
        edge > 40 && edge < 255,
        "a pixel on the curve is partly covered: {edge}"
    );
}

#[test]
fn gpu_a_shadow_darkens_below_its_box_but_not_under_it() {
    // Given: a box 10..30 casting a black shadow 4 px down, blur 4
    let caster = Rect::new(10.0, 10.0, 30.0, 30.0);
    let shadow = Shadow {
        rect: Rect::new(10.0, 14.0, 30.0, 34.0),
        radii: [0.0; 4],
        blur: 4.0,
        color: colour([0, 0, 0, 255]),
        caster,
        caster_radii: [0.0; 4],
    };

    // When
    let pixels = render(40, 40, |device, target| {
        ShadowPipeline::new(device, wgpu::TextureFormat::Rgba8Unorm).draw(target, &[shadow]);
    });

    // Then
    assert_eq!(
        pixel(&pixels, 40, 20, 20),
        BACK,
        "under the box: the box's to paint"
    );
    let below = pixel(&pixels, 40, 20, 31)[0];
    assert!(below < 30, "just below the box the shadow is dark: {below}");
    assert_eq!(pixel(&pixels, 40, 20, 2), BACK, "far above: no shadow");
    let fading = pixel(&pixels, 40, 20, 36)[0];
    assert!(
        fading > below && fading < 40,
        "it fades with distance: {fading}"
    );
}
