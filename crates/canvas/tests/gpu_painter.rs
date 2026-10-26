//! GPU tier: an image an app paints (`ImageView::set_painter`). Each
//! frame the view clears to its background and hands the painter the
//! frame, the viewport and the transform it holds after fit, zoom and
//! pan; what the painter draws is what the frame shows.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_canvas::{ImagePainter, ImageView};
use vapors_gpu_harness::{Gpu, gpu, read_rgba8};
use vapors_platform::{Event, Frame};
use vapors_renderer::{clear, clear_view};
use vapors_scene::{Color, Size, Transform};

const VIEWPORT: Size = Size {
    width: 64,
    height: 48,
};
const BACKGROUND: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 1.0,
    a: 1.0,
};
const INK: Color = Color {
    r: 1.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};

/// The (viewport, transform) of each paint, in order.
type Seen = Rc<RefCell<Vec<(Size, Transform)>>>;

/// Records what it was handed; fills the target with `ink`, if any.
struct Recorder {
    seen: Seen,
    ink: Option<Color>,
}

impl ImagePainter for Recorder {
    fn paint(
        &mut self,
        frame: &Frame<'_>,
        target: &wgpu::TextureView,
        viewport: Size,
        transform: Transform,
    ) {
        self.seen.borrow_mut().push((viewport, transform));
        if let Some(ink) = self.ink {
            clear_view(frame.device, frame.queue, target, ink);
        }
    }
}

fn painted_view(ink: Option<Color>) -> (ImageView, Seen) {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let mut view = ImageView::empty(BACKGROUND);
    view.event(&Event::Resized {
        width: VIEWPORT.width,
        height: VIEWPORT.height,
    });
    let size = Size {
        width: 640,
        height: 320,
    };
    view.set_painter(
        size,
        Box::new(Recorder {
            seen: Rc::clone(&seen),
            ink,
        }),
    );
    (view, seen)
}

/// Draw one frame of `view`; its pixels.
fn draw(gpu: &Gpu, view: &mut ImageView) -> Vec<u8> {
    let texture = clear(
        &gpu.device,
        &gpu.queue,
        VIEWPORT.width,
        VIEWPORT.height,
        INK,
    );
    view.draw(&Frame {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        texture: &texture,
        index: 0,
        scale_factor: 1.0,
    });
    read_rgba8(&gpu.device, &gpu.queue, &texture)
}

#[test]
fn gpu_the_painter_gets_the_views_transform_after_fit_zoom_and_pan() {
    // Given
    let gpu = gpu();
    let (mut view, seen) = painted_view(None);

    // When
    draw(&gpu, &mut view);
    let fitted = view.transform();
    view.zoom([10.0, 10.0], 2.0);
    draw(&gpu, &mut view);
    let zoomed = view.transform();
    view.pan(5.0, -3.0);
    draw(&gpu, &mut view);
    let panned = view.transform();

    // Then
    assert_eq!(
        *seen.borrow(),
        [(VIEWPORT, fitted), (VIEWPORT, zoomed), (VIEWPORT, panned)]
    );
    assert_ne!(fitted, zoomed);
    assert_ne!(zoomed, panned);
}

#[test]
fn gpu_the_frame_is_cleared_to_the_background_before_the_painter_draws() {
    // Given: a frame filled with ink, and a painter that draws nothing
    let gpu = gpu();
    let (mut view, _) = painted_view(None);

    // When
    let pixels = draw(&gpu, &mut view);

    // Then
    let background = BACKGROUND.to_rgba8();
    assert!(pixels.as_chunks::<4>().0.iter().all(|p| *p == background));
}

#[test]
fn gpu_what_the_painter_draws_is_what_the_frame_shows() {
    // Given
    let gpu = gpu();
    let (mut view, _) = painted_view(Some(INK));

    // When
    let pixels = draw(&gpu, &mut view);

    // Then
    let ink = INK.to_rgba8();
    assert!(pixels.as_chunks::<4>().0.iter().all(|p| *p == ink));
}
