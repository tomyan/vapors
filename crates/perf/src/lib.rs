//! The timing harness of IU1 (docs/plans/iu1.md, PR 4): the rows and
//! racer workloads run through their app's frame on a GPU, each frame
//! timed as a whole and per phase from the `tracing` spans vapors-app and
//! vapors-elements open, with the GPU's span and its time in render
//! passes (the app's `FrameTiming`) where timestamp queries
//! exist. Times are reported, never asserted; `src/main.rs` prints the
//! baseline in docs/perf.

pub mod args;
pub mod args_off_path;
pub mod circuit;
pub mod driver;
pub mod frame_kinds;
pub mod gpu_reading;
pub mod gpu_time;
pub mod instruction_samples;
pub mod instructions;
pub mod instructions_check;
pub mod mount;
pub mod mount_memory;
pub mod mount_report;
pub mod mount_run;
pub mod off_path;
pub mod off_path_kinds;
pub mod off_path_page;
pub mod off_path_scrollers;
pub mod racer;
pub mod racer_driven;
pub mod racer_input;
pub mod report;
pub mod row_change;
pub mod rows;
pub mod rows_list;
pub mod rss;
pub mod run;
pub mod sampler;
pub mod scaling;
pub mod scaling_all;
pub mod scaling_judged;
pub mod scaling_kinds;
pub mod scaling_report;
pub mod scaling_run;
pub mod scene_judge;
pub mod scene_report;
pub mod scene_run;
pub mod select;
pub mod shader_cache;
mod span_instructions;
pub mod spans;
pub mod stats;
pub mod workloads;

// The Mount workload's compiled components (ui/), at the crate root where
// the generated `use crate::mount_row` expects them.
include!(concat!(env!("OUT_DIR"), "/wisp.rs"));
