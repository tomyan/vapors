use std::collections::BTreeMap;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use super::{drained, drained_after};
use crate::api_budget::Budget;
use crate::api_curls::Curls;
use crate::api_prices::Price;
use crate::api_request_policy::Demand;

fn budget() -> Budget {
    let dollar = Price {
        input: 1.0,
        output: 1.0,
        cache_write: 1.0,
        cache_read: 1.0,
    };
    Budget::new(10.0, BTreeMap::from([("m".to_owned(), dollar)])).expect("a budget")
}

fn demand() -> Demand {
    Demand {
        model: "m".into(),
        max_tokens: 1,
        input_bound: 0,
    }
}

#[test]
fn the_drain_waits_for_a_request_settling_then_admits_no_more() {
    // Given: one request in flight that settles after 200 ms
    let budget = budget();
    let in_flight = Arc::new(AtomicUsize::new(1));
    let settling = Arc::clone(&in_flight);
    let settles = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        settling.store(0, Ordering::SeqCst);
    });

    // When
    let drained = drained(
        &budget,
        (&|| in_flight.load(Ordering::SeqCst), &Curls::default()),
        Duration::from_secs(10),
    );

    // Then: it saw the request settle before its deadline (a drain that
    // did not wait finds it in flight and fails), and is closed. No
    // wall-clock bound: one taken after the spawn failed under load
    settles.join().expect("settled");
    assert_eq!(drained, Ok(()));
    let Err(refused) = budget.admit(&demand()) else {
        panic!("admitted after the session ended");
    };
    assert!(refused.contains("the session has ended"), "{refused}");
}

#[test]
fn a_request_in_flight_at_the_deadline_has_its_curl_killed_and_fails_the_run() {
    // Given: a curl stand-in with 30 s of output to come, whose request
    // settles once its output has ended and it has been reaped
    let curls = Arc::new(Curls::default());
    let mut curl = Command::new("sleep")
        .arg("30")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .expect("started");
    curls.started(&curl);
    let in_flight = Arc::new(AtomicUsize::new(1));
    let (settling, reaping) = (Arc::clone(&in_flight), Arc::clone(&curls));
    let settles = std::thread::spawn(move || {
        let mut output = Vec::new();
        let mut stdout = curl.stdout.take().expect("piped");
        let _ = std::io::Read::read_to_end(&mut stdout, &mut output);
        let _ = reaping.reaped(&mut curl);
        settling.store(0, Ordering::SeqCst);
    });

    // When
    let started = Instant::now();
    let drained = drained(
        &budget(),
        (&|| in_flight.load(Ordering::SeqCst), &curls),
        Duration::from_millis(300),
    );

    // Then: killed at the deadline, and the run fails. The bound is
    // 20 s: under the stand-in's 30 s, so a drain that does not kill
    // still fails, and far over a loaded host's kill and reap
    settles.join().expect("settled");
    let refused = drained.expect_err("the run fails");
    assert!(
        refused.starts_with("1 API requests were still in flight"),
        "{refused}"
    );
    assert!(!refused.contains("had not settled"), "{refused}");
    assert!(
        started.elapsed() < Duration::from_secs(20),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn a_run_that_fails_is_drained_and_says_what_the_drain_left() {
    // Given: a session that failed, and requests left at the deadline
    let mut drains = 0;

    // When
    let outcome = drained_after(Err::<(), _>("the session did not start".into()), || {
        drains += 1;
        Err("1 API requests were still in flight".into())
    });

    // Then
    assert_eq!(drains, 1);
    assert_eq!(
        outcome.map(|_| ()),
        Err("the session did not start; and 1 API requests were still in flight".into())
    );
}
