//! Every option lays the timed workloads (and a few more) out to the same
//! boxes. A timing of an option that laid out something else would mean
//! nothing. Reproduce: cargo test --offline (in crates/perf/layout-spike).

#[cfg(not(all(
    feature = "taffy",
    feature = "lanes",
    feature = "fixed",
    feature = "generated"
)))]
compile_error!(
    "tests/agree.rs needs all four options, or it would check nothing; run plain \
     `cargo test --offline` in crates/perf/layout-spike, not with --no-default-features"
);

use vapors_layout_spike::agree::{check_all, expected};
use vapors_layout_spike::spec::{DEEP_DEPTH, TABLE_SIZES};
use vapors_layout_spike::{Engine, Rect, Unsupported, Workload};

fn agree(workload: Workload) {
    check_all(workload).unwrap_or_else(|why| panic!("{why}"));
}

#[test]
fn every_timed_table_is_laid_out_alike_by_every_option() {
    for rows in TABLE_SIZES {
        agree(Workload::Table(rows));
    }
}

#[test]
fn the_timed_deep_tree_is_laid_out_alike_by_every_option() {
    agree(Workload::Deep(DEEP_DEPTH));
}

#[test]
fn a_301_row_table_with_an_odd_middle_is_laid_out_alike() {
    agree(Workload::Table(301));
}

#[test]
fn a_three_level_deep_tree_is_laid_out_alike_and_matches_hand_numbers() {
    let r = |x, y, w, h| Rect { x, y, w, h };
    let (before, after) = expected(Workload::Deep(3));
    assert_eq!(
        before,
        [r(4.0, 14.0, 10.0, 10.0), r(16.0, 16.0, 10.0, 10.0)]
    );
    assert_eq!(after, [r(4.0, 14.0, 14.0, 14.0), r(20.0, 16.0, 10.0, 10.0)]);
    agree(Workload::Deep(3));
}

#[test]
fn the_deep_tree_still_moves_with_depth() {
    let (before, _) = expected(Workload::Deep(DEEP_DEPTH));
    assert!(before[0].x > 100.0 && before[1].y > 100.0, "{before:?}");
}

#[test]
fn a_boundary_refuses_a_tree_whose_sizes_are_not_static() {
    let refused = vapors_layout_spike::fixed::Fixed::build(Workload::Deep(3));
    let Err(Unsupported(why)) = refused else {
        panic!("a deep tree was taken")
    };
    assert!(why.contains("Deep"), "{why}");
}
