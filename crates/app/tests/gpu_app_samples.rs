//! GPU tier: an app's painter draws at the samples the app was given
//! (`ElementsApp::with_samples`), which are set before the first frame
//! and refused after it.

use crate::gpu_samples::{SHAPES_BOUND, difference, shapes};
use crate::support_surface as support;

use std::cell::RefCell;
use std::rc::Rc;

use support::app_frame;
use support::start_rig::{rig, start};
use vapors_app::ElementsApp;
use vapors_gpu_harness::gpu;
use vapors_renderer::Samples;

/// The shapes page's first frame by an app at `samples`.
fn first_frame(samples: Samples) -> Vec<u8> {
    let gpu = gpu();
    let mut app = ElementsApp::new(Rc::new(RefCell::new(shapes()))).with_samples(samples);
    start(&mut app, &gpu).expect("starts");
    app_frame(&mut app, &gpu).1
}

#[test]
fn gpu_an_app_given_4_samples_draws_shapes_off_the_grid_within_the_painters_bound() {
    // When
    let (one, four) = (first_frame(Samples::One), first_frame(Samples::Four));

    // Then: they differ, and by no more than the painter's measured bound
    // (`gpu_samples.rs`), so an app at 4x drawing nothing or garbage fails
    let (most, differ) = difference(&one, &four);
    assert!(differ > 0, "the app at 4 samples drew as it does at 1");
    assert!(
        most <= SHAPES_BOUND.0 && differ <= SHAPES_BOUND.1,
        "({most}, {differ}) is past the painter's measured {SHAPES_BOUND:?}"
    );
}

#[test]
#[should_panic(expected = "set_samples after the painter was made")]
fn gpu_samples_set_after_the_app_started_are_refused() {
    let (mut app, _, gpu) = rig();
    start(&mut app, &gpu).expect("starts");
    app.set_samples(Samples::Four);
}
