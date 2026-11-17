//! GPU tier: an app's painter draws at the samples the app was given
//! (`ElementsApp::with_samples`), which are set before the first frame
//! and refused after it.

use crate::gpu_samples::shapes;
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
fn gpu_an_app_given_4_samples_draws_shapes_off_the_grid_unlike_one_at_1() {
    assert_ne!(first_frame(Samples::Four), first_frame(Samples::One));
}

#[test]
#[should_panic(expected = "set_samples after the painter was made")]
fn gpu_samples_set_after_the_app_started_are_refused() {
    let (mut app, _, gpu) = rig();
    start(&mut app, &gpu).expect("starts");
    app.set_samples(Samples::Four);
}
