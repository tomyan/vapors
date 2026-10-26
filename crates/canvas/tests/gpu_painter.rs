//! GPU tier: an image an app paints (`ImageView::set_painter`). Each
//! frame the view clears to its background and hands the painter the
//! frame and the transform; a painter that draws the image's rectangle
//! by that transform lands it where the numbers say, at a 2x
//! `scale_factor` whose viewport is in device pixels; a painter that
//! fails makes the draw fail with its message.

use vapors_canvas::{ImagePainter, ImageView};
use vapors_gpu_harness::{Gpu, gpu, read_rgba8};
use vapors_platform::{Event, Frame};
use vapors_scene::{Color, Size, Transform};

/// 64 x 48 device pixels: a 32 x 24 window at 2x.
const VIEWPORT: Size = Size {
    width: 64,
    height: 48,
};
const IMAGE: Size = Size {
    width: 20,
    height: 10,
};
const BACKGROUND: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 1.0,
    a: 1.0,
};
const INK: [u8; 4] = [255, 0, 0, 255];

/// Fills the image's rectangle, `translate` to `translate + scale *
/// size`, with ink; or fails, when `fails`.
struct Rectangle {
    fails: bool,
}

impl ImagePainter for Rectangle {
    fn paint(
        &mut self,
        frame: &Frame<'_>,
        _: &wgpu::TextureView,
        _: Size,
        transform: Transform,
    ) -> Result<(), String> {
        if self.fails {
            return Err("no texture to draw from; open the image again".into());
        }
        let [x0, y0, x1, y1] = corners(transform);
        let (width, height) = (x1 - x0, y1 - y0);
        frame.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                origin: wgpu::Origin3d { x: x0, y: y0, z: 0 },
                ..frame.texture.as_image_copy()
            },
            &INK.repeat((width * height) as usize),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        Ok(())
    }
}

/// The image's corners in whole device pixels.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)] // small, positive whole numbers here
fn corners(t: Transform) -> [u32; 4] {
    let at = |v: f32| v.round() as u32;
    let size = [IMAGE.width as f32, IMAGE.height as f32];
    [
        at(t.translate[0]),
        at(t.translate[1]),
        at(t.translate[0] + t.scale * size[0]),
        at(t.translate[1] + t.scale * size[1]),
    ]
}

fn painted_view(fails: bool) -> ImageView {
    let mut view = ImageView::empty(BACKGROUND);
    view.event(&Event::Resized {
        width: VIEWPORT.width,
        height: VIEWPORT.height,
    });
    view.set_painter(IMAGE, Box::new(Rectangle { fails }));
    view
}

/// One frame of `view` at 2x; the draw's result and the pixels.
fn draw(gpu: &Gpu, view: &mut ImageView) -> (Result<(), String>, Vec<u8>) {
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("test frame"),
        size: wgpu::Extent3d {
            width: VIEWPORT.width,
            height: VIEWPORT.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let drawn = view.draw(&Frame {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        texture: &texture,
        index: 0,
        scale_factor: 2.0,
        now: vapors_platform::Instant::now(),
    });
    (drawn, read_rgba8(&gpu.device, &gpu.queue, &texture))
}

/// The inked pixels' bounds: [x0, y0, x1, y1), exclusive ends.
fn ink_bounds(pixels: &[u8]) -> Option<[u32; 4]> {
    let inked: Vec<(u32, u32)> = (0..VIEWPORT.width * VIEWPORT.height)
        .filter(|i| pixels[(*i * 4) as usize..][..4] == INK)
        .map(|i| (i % VIEWPORT.width, i / VIEWPORT.width))
        .collect();
    let xs = inked.iter().map(|p| p.0);
    let ys = inked.iter().map(|p| p.1);
    Some([
        xs.clone().min()?,
        ys.clone().min()?,
        xs.max()? + 1,
        ys.max()? + 1,
    ])
}

#[test]
fn gpu_the_painted_image_lands_where_fit_zoom_and_pan_place_it() {
    // Given: 20 x 10 in 64 x 48 fits at 3.2, centred 8 px down
    let gpu = gpu();
    let mut view = painted_view(false);
    let fitted = Transform {
        scale: 3.2,
        translate: [0.0, 8.0],
    };
    assert_eq!(view.transform(), fitted);

    // When / Then: fitted, the image covers x 0..64, y 8..40
    let (drawn, pixels) = draw(&gpu, &mut view);
    assert_eq!(drawn, Ok(()));
    assert_eq!(ink_bounds(&pixels), Some([0, 8, 64, 40]));

    // When / Then: halved about (10, 10), scale 1.6 at (5, 9)
    view.zoom([10.0, 10.0], 0.5);
    let (_, pixels) = draw(&gpu, &mut view);
    assert_eq!(ink_bounds(&pixels), Some([5, 9, 37, 25]));

    // When / Then: moved by (3, -2), at (8, 7)
    view.pan(3.0, -2.0);
    let (_, pixels) = draw(&gpu, &mut view);
    assert_eq!(ink_bounds(&pixels), Some([8, 7, 40, 23]));
}

#[test]
fn gpu_the_frame_is_cleared_to_the_background_before_the_painter_draws() {
    // Given
    let gpu = gpu();
    let mut view = painted_view(false);

    // When
    let (_, pixels) = draw(&gpu, &mut view);

    // Then: outside the image, the background
    assert_eq!(pixels[..4], BACKGROUND.to_rgba8());
}

#[test]
fn gpu_a_painter_that_fails_fails_the_draw_with_its_message() {
    // Given
    let gpu = gpu();
    let mut view = painted_view(true);

    // When
    let (drawn, _) = draw(&gpu, &mut view);

    // Then
    assert_eq!(
        drawn,
        Err("image painter: no texture to draw from; open the image again".into())
    );
}
