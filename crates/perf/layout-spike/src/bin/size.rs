//! The program the wasm size is measured on: it lays a table out by
//! each option compiled in (a feature per option) and nothing else, so
//! the size of a build with a feature less than another is that
//! option's cost. Build it with `size.sh`; docs/plans/layout-comparison.md.

use std::hint::black_box;

use vapors_layout_spike::{Engine, Workload};

#[allow(dead_code)] // the build with no option compiled in has no caller
/// Builds, lays out, changes and lays out again, and keeps the boxes.
fn run<E: Engine>(workload: Workload) {
    let mut engine = E::build(black_box(workload)).expect("the table is taken");
    engine.layout();
    if engine.change_leaf().is_ok() {
        engine.layout();
    }
    black_box(engine.probe());
}

fn main() {
    let table = Workload::Table(black_box(30));
    #[cfg(feature = "taffy")]
    run::<vapors_layout_spike::taffy_engine::Taffy>(table);
    #[cfg(feature = "lanes")]
    run::<vapors_layout_spike::lanes::Lanes>(table);
    #[cfg(feature = "fixed")]
    run::<vapors_layout_spike::fixed::Fixed>(table);
    #[cfg(feature = "generated")]
    run::<vapors_layout_spike::generated::Generated>(table);
    let _ = table;
}
