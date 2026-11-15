//! `wake_window`'s burst judgement with injected timing: the bound from
//! the gap and the frame interval, the retry while bursts spread out,
//! and each refusal. The window itself runs in the CI window job.

#[path = "../examples/wake_window/burst_judge.rs"]
mod burst_judge;

use std::convert::Infallible;
use std::time::Duration;

use burst_judge::{ATTEMPTS, Attempt, Outcome, Pair, interval, judge, most};

const F: Duration = Duration::from_millis(1);

fn burst(frames: u64, gap: Duration) -> Attempt {
    Attempt {
        frames,
        gap,
        closest: None,
    }
}

/// Judge `attempts` in order, as if each burst measured them.
fn judged(attempts: &[Attempt]) -> (Vec<Attempt>, Outcome) {
    let mut scripted = attempts.iter().copied();
    let next = || Ok::<_, Infallible>(scripted.next().expect("judge asked for one burst too many"));
    match judge(F, next) {
        Ok(judged) => judged,
        Err(never) => match never {},
    }
}

fn refusal(outcome: &Outcome) -> &str {
    match outcome {
        Outcome::Refused(why) => why,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn the_bound_is_one_more_than_the_frame_intervals_the_gap_spans() {
    let cases = [
        (Duration::ZERO, 1),
        (F / 2, 2),
        (F, 2),
        (F * 3, 4),
        (F * 3 + Duration::from_nanos(1), 5),
    ];
    for (gap, expected) in cases {
        assert_eq!(most(gap, F), expected, "gap {gap:?}");
    }
}

#[test]
fn a_burst_within_its_bound_holds_on_the_first_attempt() {
    // When
    let (attempts, outcome) = judged(&[burst(2, F / 2)]);

    // Then
    assert_eq!(outcome, Outcome::Held);
    assert_eq!(attempts.len(), 1);
}

#[test]
fn frames_outside_the_bound_fail_the_count_and_are_never_retried() {
    for frames in [0, 3] {
        // When: a gap under one interval allows 1 or 2
        let (attempts, outcome) = judged(&[burst(frames, F / 2), burst(1, F / 2)]);

        // Then
        let expected = format!("expected 1 to 2 frames for the burst, got {frames}");
        assert_eq!(outcome, Outcome::Count(expected));
        assert_eq!(attempts.len(), 1);
    }
}

#[test]
fn a_burst_that_spread_out_is_tried_again() {
    // Given: the first burst's gap spans more than 3 intervals
    let spread = burst(5, F * 4);

    // When
    let (attempts, outcome) = judged(&[spread, burst(1, F / 2)]);

    // Then
    assert_eq!(outcome, Outcome::Held);
    assert_eq!(attempts.len(), 2);
}

#[test]
fn bursts_that_spread_out_every_attempt_are_refused_naming_each_gap() {
    // Given
    let gaps = [F * 4, F * 5, F * 6];

    // When
    let (attempts, outcome) = judged(&gaps.map(|gap| burst(2, gap)));

    // Then
    assert_eq!(attempts.len(), ATTEMPTS);
    let why = refusal(&outcome);
    assert!(why.contains("[4ms, 5ms, 6ms]"), "{why}");
    assert!(why.contains("frame intervals of 1ms"), "{why}");
    assert!(why.contains("re-run on a quieter machine"), "{why}");
}

#[test]
fn burst_frames_closer_than_the_interval_are_refused() {
    // Given: two woken frames half an interval apart
    let close = Attempt {
        closest: Some(F / 2),
        ..burst(2, F / 2)
    };

    // When
    let (_, outcome) = judged(&[close]);

    // Then
    let why = refusal(&outcome);
    assert!(
        why.contains("500µs apart, under the frame interval of 1ms"),
        "{why}"
    );
}

#[test]
fn the_interval_is_the_shortest_back_to_back_pair() {
    // Given
    let pairs = [3, 1, 2].map(|ms| Pair {
        drawn: 2,
        apart: Some(Duration::from_millis(ms)),
    });

    // Then
    assert_eq!(interval(&pairs), Ok(Duration::from_millis(1)));
}

#[test]
fn an_interval_of_zero_is_refused_on_its_own() {
    // Given
    let pairs = [Pair {
        drawn: 2,
        apart: Some(Duration::ZERO),
    }];

    // When
    let why = interval(&pairs).expect_err("a zero interval must be refused");

    // Then
    assert!(why.contains("burst was not run"), "{why}");
    assert!(why.contains("0ns apart"), "{why}");
}

#[test]
fn an_interval_from_a_pair_that_did_not_draw_two_frames_is_refused() {
    for (drawn, apart) in [(1, None), (3, Some(F))] {
        // Given: one good pair and one that drew `drawn` frames
        let good = Pair {
            drawn: 2,
            apart: Some(F),
        };
        let pairs = [good, Pair { drawn, apart }];

        // When
        let why = interval(&pairs).expect_err("an unmeasured pair must be refused");

        // Then
        assert!(
            why.contains(&format!("drew {drawn} frames, not 2")),
            "{why}"
        );
    }
}

#[test]
fn an_interval_with_no_pairs_is_refused() {
    let why = interval(&[]).expect_err("no pairs must be refused");
    assert!(why.contains("no back-to-back pair was measured"), "{why}");
}
