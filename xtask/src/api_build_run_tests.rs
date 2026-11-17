use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use super::waited;

#[test]
fn the_summary_waits_for_a_request_still_in_flight_to_settle() {
    // Given: one request in flight that settles after 200 ms
    let in_flight = Arc::new(AtomicUsize::new(1));
    let settling = Arc::clone(&in_flight);
    let settles = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        settling.store(0, Ordering::SeqCst);
    });

    // When
    let started = Instant::now();
    let none_left = waited(
        &|| in_flight.load(Ordering::SeqCst),
        Duration::from_secs(10),
    );

    // Then: it waited for the settling, not the deadline
    settles.join().expect("settled");
    assert!(none_left);
    assert!(
        started.elapsed() >= Duration::from_millis(200),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn a_request_that_never_settles_is_left_at_the_deadline() {
    let none_left = waited(&|| 1, Duration::from_millis(50));
    assert!(!none_left);
}
