//! A workload drawn at another size than its own (`--size`): every
//! other answer is the workload's.

use vapors_app::FrameTiming;
use vapors_platform::{App, Next};

use crate::run::{Counts, Workload};

/// `inner`, its frames `size` pixels.
pub struct Resized {
    inner: Box<dyn Workload>,
    size: (u32, u32),
}

impl Resized {
    /// `inner` drawn at `size`.
    #[must_use]
    pub fn new(inner: Box<dyn Workload>, size: (u32, u32)) -> Self {
        Self { inner, size }
    }
}

impl Workload for Resized {
    fn size(&self) -> (u32, u32) {
        self.size
    }
    fn change(&mut self, sample: usize) {
        self.inner.change(sample);
    }
    fn app(&mut self) -> &mut dyn App {
        self.inner.app()
    }
    fn check(&self, next: Next) -> Result<(), String> {
        self.inner.check(next)
    }
    fn counts(&self) -> Counts {
        self.inner.counts()
    }
    fn timing(&self) -> FrameTiming {
        self.inner.timing()
    }
    fn ready(&self) -> Result<(), String> {
        self.inner.ready()
    }
    fn frame_kinds(&self) -> &'static [&'static str] {
        self.inner.frame_kinds()
    }
}
