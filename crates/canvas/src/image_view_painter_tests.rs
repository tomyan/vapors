use std::cell::Cell;
use std::rc::Rc;

use vapors_platform::{Event, Frame};
use vapors_scene::{Color, RgbaImage, Size, Transform};

use super::ImagePainter;
use crate::ImageView;

/// Paints nothing; counts how many painters are alive.
struct Idle(Rc<Cell<u32>>);

impl Idle {
    fn boxed(alive: &Rc<Cell<u32>>) -> Box<dyn ImagePainter> {
        alive.set(alive.get() + 1);
        Box::new(Self(Rc::clone(alive)))
    }
}

impl Drop for Idle {
    fn drop(&mut self) {
        self.0.set(self.0.get() - 1);
    }
}

impl ImagePainter for Idle {
    fn paint(&mut self, _: &Frame<'_>, _: &wgpu::TextureView, _: Size, _: Transform) {}
}

const SIZE: Size = Size {
    width: 400,
    height: 100,
};

fn resized(view: &mut ImageView) {
    view.event(&Event::Resized {
        width: 200,
        height: 200,
    });
}

#[test]
fn a_painted_image_is_fitted_by_its_size() {
    // Given
    let mut view = ImageView::empty(Color::default());
    resized(&mut view);

    // When
    view.set_painter(SIZE, Idle::boxed(&Rc::new(Cell::new(0))));

    // Then
    assert!(view.has_image());
    assert_eq!(
        view.transform(),
        Transform::fit(
            SIZE,
            Size {
                width: 200,
                height: 200
            }
        )
    );
}

#[test]
fn setting_a_painter_counts_as_a_new_image() {
    // Given
    let mut view = ImageView::empty(Color::default());
    let before = view.generation();

    // When
    view.set_painter(SIZE, Idle::boxed(&Rc::new(Cell::new(0))));

    // Then
    assert_eq!(view.generation(), before + 1);
}

#[test]
fn pixels_set_after_a_painter_replace_it() {
    // Given
    let alive = Rc::new(Cell::new(0));
    let mut view = ImageView::empty(Color::default());
    view.set_painter(SIZE, Idle::boxed(&alive));

    // When
    view.set_image(RgbaImage {
        size: Size {
            width: 1,
            height: 1,
        },
        rgba: vec![0; 4],
    });

    // Then
    assert_eq!(alive.get(), 0, "the painter is dropped");
}
