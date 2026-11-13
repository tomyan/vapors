//! A meter sums each value of its counter by phase between a span's
//! enter and exit, starts again from none after a take, and gives a
//! failed read at the take. A scripted counter stands in for the
//! hardware, so no perf event is needed.

use std::collections::VecDeque;

use super::{Counting, Meter};

/// A counter that reads the scripted readings in order.
struct Scripted(VecDeque<Result<[u64; 2], String>>);

impl Counting<2> for Scripted {
    fn read_all(&mut self) -> Result<[u64; 2], String> {
        self.0
            .pop_front()
            .unwrap_or_else(|| Err("no more readings".into()))
    }
}

fn meter(readings: &[[u64; 2]]) -> Meter<Scripted, 2> {
    Meter::new(Scripted(readings.iter().copied().map(Ok).collect()))
}

#[test]
fn each_value_sums_by_phase_over_nested_and_repeated_spans() {
    // Given: update holds style; style runs twice
    let mut meter = meter(&[[0, 0], [10, 1], [30, 4], [40, 5], [70, 9], [100, 20]]);

    // When
    meter.enter(1); // update at [0, 0]
    meter.enter(2); // style at [10, 1]
    meter.exit(2, "style"); // [30, 4]
    meter.enter(3); // style at [40, 5]
    meter.exit(3, "style"); // [70, 9]
    meter.exit(1, "update"); // [100, 20]

    // Then
    let totals = meter.take().expect("read");
    assert_eq!(totals["style"], [50, 7]);
    assert_eq!(totals["update"], [100, 20]);
}

#[test]
fn a_take_starts_the_sums_again_from_none() {
    let mut meter = meter(&[[0, 0], [5, 5]]);
    meter.enter(1);
    meter.exit(1, "draw");
    assert_eq!(meter.take().expect("read")["draw"], [5, 5]);
    assert!(meter.take().expect("read").is_empty());
}

#[test]
fn a_failed_read_is_given_at_the_take() {
    // Given: the read at the exit fails
    let mut meter = Meter::new(Scripted(VecDeque::from([Ok([0, 0]), Err("gone".into())])));

    // When
    meter.enter(1);
    meter.exit(1, "draw");

    // Then
    assert_eq!(meter.take(), Err("gone".to_owned()));
}
