//! The viewport as signals a script reads: `viewport_size()` and
//! `orientation()`, two app-wide signals the app sets
//! (`set_viewport_size`) only when the size changes, so they never ask
//! for a frame themselves. They belong to no owner, as the frame
//! clock's do. The primary input (`pointer` and `hover` in `@media`)
//! is set once at start (`set_primary_input`).

use std::cell::{Cell, RefCell};

use vapors_css::cascade::{Environment, Pointer};
use vapors_signals::{ReadSignal, Signal, unowned};

/// The drawing area's size in CSS pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Size {
    /// Width in CSS pixels.
    pub width: f32,
    /// Height in CSS pixels.
    pub height: f32,
}

/// Which way the drawing area is longer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    /// Taller than wide.
    Portrait,
    /// Wider than tall, or square.
    Landscape,
}

/// The primary input, as `@media (pointer)` and `(hover)` see it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrimaryInput {
    /// The primary pointing device.
    pub pointer: Pointer,
    /// Whether it can hover.
    pub hover: bool,
}

impl PrimaryInput {
    /// A mouse: `fine` and `hover`, what a native window has.
    pub const MOUSE: Self = Self {
        pointer: Pointer::Fine,
        hover: true,
    };

    /// From the browser's `matchMedia` answers to `(pointer: coarse)`,
    /// `(pointer: fine)` and `(hover: hover)`.
    #[must_use]
    pub fn from_media(coarse: bool, fine: bool, hover: bool) -> Self {
        let pointer = match (coarse, fine) {
            (true, _) => Pointer::Coarse,
            (false, true) => Pointer::Fine,
            (false, false) => Pointer::None,
        };
        Self { pointer, hover }
    }
}

const SET: &str = "set_viewport_size was called inside a running effect that reads the viewport; call it from the app's frame, outside effects";

#[derive(Clone, Copy)]
struct Viewport {
    size: Signal<Size>,
    orientation: Signal<Orientation>,
}

thread_local! {
    static VIEWPORT: RefCell<Option<Viewport>> = const { RefCell::new(None) };
    static INPUT: Cell<PrimaryInput> = const { Cell::new(PrimaryInput::MOUSE) };
}

fn viewport() -> Viewport {
    VIEWPORT.with(|slot| {
        *slot.borrow_mut().get_or_insert_with(|| {
            unowned(|| Viewport {
                size: Signal::named(
                    "viewport_size",
                    Size {
                        width: 0.0,
                        height: 0.0,
                    },
                ),
                orientation: Signal::named("orientation", Orientation::Landscape),
            })
        })
    })
}

/// The drawing area's size in CSS px (the canvas on the web, the
/// window's content natively): the numbers layout and `@media` use.
/// Zero until the app first sets it.
#[must_use]
pub fn viewport_size() -> ReadSignal<Size> {
    viewport().size.read_only()
}

/// Portrait when taller than wide; a square is landscape. The rule is
/// `@media (orientation)`'s, from the same viewport.
#[must_use]
pub fn orientation() -> ReadSignal<Orientation> {
    viewport().orientation.read_only()
}

/// Sets both signals for a drawing area of `width` by `height` CSS px.
/// Readers are told only when a value changed.
///
/// # Panics
/// When called inside a running effect that reads the signals.
pub fn set_viewport_size(width: f32, height: f32) {
    let now = viewport();
    let portrait = environment_of(crate::Viewport {
        width,
        height,
        scale: 1.0,
    })
    .is_portrait();
    now.size
        .try_set_if_changed(Size { width, height })
        .expect(SET);
    let turn = if portrait {
        Orientation::Portrait
    } else {
        Orientation::Landscape
    };
    now.orientation.try_set_if_changed(turn).expect(SET);
}

/// Sets the primary input every later update's `@media` reads.
pub fn set_primary_input(input: PrimaryInput) {
    INPUT.with(|slot| slot.set(input));
}

pub(crate) fn primary_input() -> PrimaryInput {
    INPUT.with(Cell::get)
}

/// The environment `@media` reads for `viewport`.
pub(crate) fn environment_of(viewport: crate::Viewport) -> Environment {
    let input = primary_input();
    Environment {
        viewport_width: f64::from(viewport.width),
        viewport_height: f64::from(viewport.height),
        resolution: f64::from(viewport.scale),
        pointer: input.pointer,
        hover: input.hover,
        ..Environment::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_environment_carries_the_primary_input() {
        let view = crate::Viewport {
            width: 1.0,
            height: 1.0,
            scale: 1.0,
        };
        assert_eq!(environment_of(view).pointer, Pointer::Fine);
        set_primary_input(PrimaryInput::from_media(true, false, false));
        let env = environment_of(view);
        assert_eq!((env.pointer, env.hover), (Pointer::Coarse, false));
    }
}
