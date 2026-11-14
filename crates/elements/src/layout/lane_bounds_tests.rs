//! A run followed by another: the right run's offsets moved by where it
//! starts, the first baseline kept, the cross contributions the larger.

use super::Bounds;

fn bounds(low: f64, reach: f64, baseline: Option<f64>, cross: [f64; 2]) -> Bounds {
    Bounds {
        low,
        reach,
        end: reach,
        across: [0.0, 0.0],
        across_end: 0.0,
        baseline,
        cross,
    }
}

#[test]
fn the_earliest_end_is_the_least_of_the_childrens_ends() {
    let left = bounds(0.0, 10.0, None, [0.0; 2]);
    let right = bounds(0.0, 4.0, None, [0.0; 2]);
    assert_eq!(left.then(-20.0, &right).end, -16.0);
    assert_eq!(left.then(20.0, &right).end, 10.0);
}

#[test]
fn empty_is_the_identity_on_either_side() {
    let run = bounds(-2.0, 5.0, Some(3.0), [1.0, 2.0]);
    assert_eq!(
        Bounds::EMPTY.then(7.0, &run),
        bounds(5.0, 12.0, Some(10.0), [1.0, 2.0])
    );
    assert_eq!(run.then(7.0, &Bounds::EMPTY), run);
}

#[test]
fn the_reach_across_the_axis_is_not_moved_by_the_offset() {
    let left = Bounds {
        across: [-1.0, 10.0],
        ..bounds(0.0, 5.0, None, [0.0; 2])
    };
    let right = Bounds {
        across: [2.0, 12.0],
        ..bounds(0.0, 5.0, None, [0.0; 2])
    };
    assert_eq!(left.then(100.0, &right).across, [-1.0, 12.0]);
}

#[test]
fn the_right_runs_offsets_are_moved_by_where_it_starts() {
    // Given
    let left = bounds(0.0, 10.0, None, [5.0, 20.0]);
    let right = bounds(-3.0, 8.0, Some(4.0), [7.0, 30.0]);

    // When: the right run starts 6 after the left
    let run = left.then(6.0, &right);

    // Then: the earliest end is the left run's, 10, before the right's 14
    let expected = Bounds {
        end: 10.0,
        ..bounds(0.0, 14.0, Some(10.0), [7.0, 30.0])
    };
    assert_eq!(run, expected);
}

#[test]
fn the_left_runs_baseline_and_low_are_kept_when_first() {
    let left = bounds(-2.0, 5.0, Some(3.0), [0.0, 0.0]);
    let right = bounds(0.0, 5.0, Some(1.0), [0.0, 0.0]);
    let run = left.then(10.0, &right);
    assert_eq!((run.low, run.baseline), (-2.0, Some(3.0)));
}
