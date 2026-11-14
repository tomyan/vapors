//! A run followed by another: the right run's offsets moved by where it
//! starts, the first baseline kept, the cross contributions the larger.

use super::Bounds;

fn bounds(low: f64, reach: f64, baseline: Option<f64>, cross: [f64; 2]) -> Bounds {
    Bounds {
        low,
        reach,
        baseline,
        cross,
    }
}

#[test]
fn the_right_runs_offsets_are_moved_by_where_it_starts() {
    // Given
    let left = bounds(0.0, 10.0, None, [5.0, 20.0]);
    let right = bounds(-3.0, 8.0, Some(4.0), [7.0, 30.0]);

    // When: the right run starts 6 after the left
    let run = left.then(6.0, &right);

    // Then
    assert_eq!(run, bounds(0.0, 14.0, Some(10.0), [7.0, 30.0]));
}

#[test]
fn the_left_runs_baseline_and_low_are_kept_when_first() {
    let left = bounds(-2.0, 5.0, Some(3.0), [0.0, 0.0]);
    let right = bounds(0.0, 5.0, Some(1.0), [0.0, 0.0]);
    let run = left.then(10.0, &right);
    assert_eq!((run.low, run.baseline), (-2.0, Some(3.0)));
}
