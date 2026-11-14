//! GPU tier: one page measured for property 8 under a counting
//! allocator: its live heap split by owner, summing to its total and
//! within the split's limits (`ElementMemory::checked`), its elements
//! counted, and the mount, layout and paint each charged. The only test
//! in this file: a second would allocate on another thread against the
//! same process-wide count.

use std::alloc::System;

use stats_alloc::{INSTRUMENTED_SYSTEM, StatsAlloc};
use vapors_gpu_harness::gpu;
use vapors_perf::element_memory::OWNERS;
use vapors_perf::element_memory_run::{live_bytes, run_child};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn live() -> i64 {
    live_bytes(GLOBAL)
}

#[test]
fn gpu_a_settled_page_s_live_heap_is_charged_to_its_mount_layout_and_paint() {
    // Given
    let gpu = gpu();

    // When: an empty page, then 20 rows, each passing ElementMemory::checked
    let empty = run_child(&gpu, 0, live).expect("an empty page mounts and settles");
    let measured = run_child(&gpu, 20, live).expect("mounts and settles");

    // Then: each row is four elements, under the html and body
    assert_eq!((empty.elements, measured.elements), (2, 20 * 4 + 2));
    let owner =
        |name: &str| measured.owners[OWNERS.iter().position(|o| *o == name).expect("an owner")];
    for name in ["mount", "layout", "paint"] {
        assert!(owner(name) > 0, "{name}: {measured:?}");
    }
    // the readings at 20 rows: layout 744 KB (the boxes), paint 176 KB,
    // style 97 KB; a split that swaps any two of them fails here
    assert!(owner("layout") > owner("paint"), "{measured:?}");
    assert!(owner("paint") > owner("style"), "{measured:?}");
    assert!(measured.total > empty.total, "{empty:?} {measured:?}");
}
