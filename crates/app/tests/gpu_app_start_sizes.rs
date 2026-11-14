//! GPU tier: `App::start` lays the document out at the start's size and
//! makes each surface's backing at its laid-out size (one-scene F0); a
//! frame 0 at another size remakes them, and says so.

#[path = "support/surface.rs"]
mod support;

use std::cell::RefCell;
use std::rc::Rc;
use support::start_rig::{rig, start, start_at};

use support::{RED, SIZE, Solid, app_frame, document};
use vapors_app::ElementsApp;
use vapors_app::{StartState, SurfaceSize};
use vapors_gpu_harness::gpu;
use vapors_platform::{App, Start};
use vapors_scene::Size;

#[test]
fn gpu_start_lays_the_document_out_and_sizes_each_surface_before_the_first_frame() {
    // Given
    let (mut app, log, gpu) = rig();
    start(&mut app, &gpu).expect("starts");
    let at_start = log.borrow().resizes.clone();

    // When
    app_frame(&mut app, &gpu);

    // Then: the first frame reuses the backing sized at start
    let side = SurfaceSize {
        width: 16,
        height: 16,
        scale: 1.0,
    };
    assert_eq!(at_start, vec![side]);
    assert_eq!(log.borrow().resizes, vec![side]);
    assert_eq!(log.borrow().frames, vec![side]);
}

#[test]
fn gpu_a_start_with_no_pixels_on_either_side_is_refused_naming_the_size_and_the_fix() {
    for (width, height, named) in [(0, SIZE, "0x32"), (SIZE, 0, "32x0")] {
        // Given
        let (mut app, _, gpu) = rig();

        // When
        let refused = app.start(&Start {
            width,
            height,
            ..start_at(&gpu)
        });

        // Then
        let error = refused.expect_err("a surface with no pixels has no layout");
        assert!(error.contains(named) && error.contains("fix:"), "{error}");
    }
}

#[test]
fn gpu_a_first_frame_at_another_scale_than_the_start_remakes_the_backing_and_says_so() {
    // Given: started at scale 2, so the 16 px surface is 32 device px
    let (mut app, log, gpu) = rig();
    let started = app.start(&Start {
        scale_factor: 2.0,
        ..start_at(&gpu)
    });
    started.expect("starts");

    // When: frame 0 at scale 1
    app_frame(&mut app, &gpu);

    // Then
    let side = |width, scale| SurfaceSize {
        width,
        height: width,
        scale,
    };
    assert_eq!(log.borrow().resizes, vec![side(32, 2.0), side(16, 1.0)]);
    let target = Size {
        width: SIZE,
        height: SIZE,
    };
    let other = StartState::StartedAtOtherSize {
        start: (target, 2.0),
        frame: (target, 1.0),
    };
    assert_eq!(app.start_state(), other);
}

#[test]
fn gpu_a_start_with_two_surfaces_of_one_name_is_refused_naming_it() {
    // Given
    let elements = [
        (None, "gpu-surface", "track"),
        (None, "gpu-surface", "track"),
    ];
    let doc = document(".track { width: 8px; height: 8px; }", &elements);
    let mut app = ElementsApp::new(Rc::new(RefCell::new(doc)));
    let (solid, _) = Solid::new(RED);
    app.mount("track", Box::new(solid)).expect("mounts");
    let gpu = gpu();

    // When
    let refused = app.start(&start_at(&gpu));

    // Then
    let error = refused.expect_err("two surfaces named track");
    assert!(error.contains("track"), "{error}");
}
