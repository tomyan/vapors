//! `viewport_size()` and `orientation()`: app-wide signals the app sets
//! when the size changes, the same viewport `@media` reads.

use std::cell::Cell;
use std::rc::Rc;

use vapors_css::cascade::{Environment, Pointer};
use vapors_elements::{
    Orientation, PrimaryInput, Size, orientation, set_viewport_size, viewport_size,
};
use vapors_signals::Effect;

#[test]
fn a_resize_updates_both_and_the_same_size_does_nothing() {
    let runs = Rc::new(Cell::new(0));
    let counted = Rc::clone(&runs);
    let (size, turn) = (viewport_size(), orientation());
    let _watch = Effect::new(move || {
        let _ = (size.get(), turn.get());
        counted.set(counted.get() + 1);
    });
    set_viewport_size(400.0, 800.0);
    assert_eq!(
        size.get_untracked(),
        Size {
            width: 400.0,
            height: 800.0
        }
    );
    assert_eq!(turn.get_untracked(), Orientation::Portrait);
    let after_resize = runs.get();
    set_viewport_size(400.0, 800.0);
    assert_eq!(runs.get(), after_resize, "an unchanged size notified");
    set_viewport_size(400.0, 900.0);
    assert_eq!(runs.get(), after_resize + 1);
}

#[test]
fn orientation_and_the_media_query_agree_at_portrait_landscape_and_square() {
    for (w, h) in [(400.0_f32, 800.0_f32), (800.0, 400.0), (500.0, 500.0)] {
        set_viewport_size(w, h);
        let env = Environment {
            viewport_width: f64::from(w),
            viewport_height: f64::from(h),
            ..Environment::default()
        };
        let portrait = orientation().get_untracked() == Orientation::Portrait;
        assert_eq!(portrait, env.is_portrait(), "{w}x{h}");
        assert_eq!(portrait, h > w, "{w}x{h}");
    }
}

#[test]
fn the_web_maps_matchmedia_answers_to_the_primary_input() {
    let input = |coarse, fine, hover| PrimaryInput::from_media(coarse, fine, hover);
    assert_eq!(
        input(true, false, false),
        PrimaryInput {
            pointer: Pointer::Coarse,
            hover: false
        }
    );
    assert_eq!(
        input(false, true, true),
        PrimaryInput {
            pointer: Pointer::Fine,
            hover: true
        }
    );
    assert_eq!(input(false, false, false).pointer, Pointer::None);
}
