//! GPU tier: one page measured for property 8 under a counting
//! allocator: its live heap split by owner, its elements counted, and
//! the mount, layout and paint each charged.

use std::alloc::System;

use stats_alloc::{INSTRUMENTED_SYSTEM, StatsAlloc};
use vapors_gpu_harness::gpu;
use vapors_perf::element_memory::OWNERS;
use vapors_perf::element_memory_run::run_child;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn live() -> i64 {
    let stats = GLOBAL.stats();
    i64::try_from(stats.bytes_allocated).expect("fits")
        - i64::try_from(stats.bytes_deallocated).expect("fits")
}

#[test]
fn gpu_a_settled_page_s_live_heap_is_charged_to_its_mount_layout_and_paint() {
    // Given
    let gpu = gpu();

    // When
    let measured = run_child(&gpu, 20, live).expect("mounts and settles");

    // Then: each row is four elements, under the html and body
    assert_eq!(measured.elements, 20 * 4 + 2);
    for owner in ["mount", "layout", "paint"] {
        let at = OWNERS
            .iter()
            .position(|name| *name == owner)
            .expect("an owner");
        assert!(measured.owners[at] > 0, "{owner}: {measured:?}");
    }
    assert!(measured.total > 0, "{measured:?}");
}
