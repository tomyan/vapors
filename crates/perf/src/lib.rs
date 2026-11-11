//! The timing harness of IU1 (docs/plans/iu1.md, PR 4): the rows and
//! racer workloads run through their app's frame on a GPU, each frame
//! timed as a whole and per phase from the `tracing` spans vapors-app and
//! vapors-elements open, with the GPU's span and its time in render
//! passes (the app's `FrameTiming`) where timestamp queries
//! exist. Times are reported, never asserted; `src/main.rs` prints the
//! baseline in docs/perf.

pub mod args;
pub mod gpu_reading;
pub mod gpu_time;
pub mod racer;
pub mod report;
pub mod rows;
pub mod run;
pub mod sampler;
pub mod spans;
pub mod stats;
