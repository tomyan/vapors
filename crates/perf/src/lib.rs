//! The timing harness of IU1 (docs/plans/iu1.md, PR 4): the rows and
//! racer workloads run through their app's frame on a GPU, each frame
//! timed as a whole and per phase from the `tracing` spans vapors-app and
//! vapors-elements open, with the GPU's span and its time in render
//! passes (the app's `FrameTiming`) where timestamp queries
//! exist. Times are reported, never asserted; `src/main.rs` prints the
//! baseline in docs/perf.

pub mod args;
pub mod args_measure;
pub mod args_off_path;
mod args_samples;
mod args_scene_profile;
pub mod build_profile;
pub mod child_deadline;
pub mod circuit;
pub mod counted_frames;
pub mod counter_samples;
pub mod counters_report;
pub mod driver;
pub mod element_memory;
pub mod element_memory_add;
pub mod element_memory_flat;
pub mod element_memory_run;
pub mod element_memory_sizes;
pub mod first_frame_report;
pub mod first_frame_run;
pub mod frame_counts;
pub mod frame_kinds;
pub mod gpu_reading;
pub mod gpu_time;
pub mod hardware_counters;
pub mod heap_phases;
pub mod idle;
pub mod idle_run;
pub mod instruction_samples;
pub mod instructions;
pub mod instructions_check;
pub mod llc_line;
pub mod mount;
pub mod mount_memory;
pub mod mount_run;
pub mod off_path;
pub mod off_path_kinds;
pub mod off_path_layouts;
pub mod off_path_moved;
pub mod off_path_page;
pub mod off_path_scrollers;
pub mod one_scene_driven;
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
pub mod scaling_reported;
pub mod scaling_run;
pub mod scene_judge;
pub mod scene_kind;
pub mod scene_report;
mod scene_report_phases;
pub mod scene_run;
pub mod scene_size;
pub mod select;
pub mod shader_cache;
mod span_meter;
pub mod spans;
pub mod splice_tally;
pub mod stats;
pub mod workloads;

// The Mount workload's compiled components (ui/), at the crate root where
// the generated `use crate::mount_row` expects them.
include!(concat!(env!("OUT_DIR"), "/wisp.rs"));
