//! The wait for the machine's load, on a fake clock and load.

use std::cell::Cell;

use super::*;

/// A clock that moves only when slept on.
struct FakeClock {
    start: Instant,
    elapsed: Cell<Duration>,
}

impl FakeClock {
    fn new() -> Self {
        Self {
            start: Instant::now(),
            elapsed: Cell::new(Duration::ZERO),
        }
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Instant {
        self.start + self.elapsed.get()
    }
    fn sleep(&self, period: Duration) {
        self.elapsed.set(self.elapsed.get() + period);
    }
}

const SETTINGS: Settings = Settings {
    enabled: true,
    threshold: 12.0,
    deadline: Duration::from_secs(600),
};

/// The wait over `loads`, read in turn (the last repeating): how it
/// ended, what it said, and how long it slept.
fn run(loads: &[f64], github: bool) -> (Result<Outcome, String>, Vec<String>, Duration) {
    let clock = FakeClock::new();
    let mut read = 0;
    let mut load = || {
        let value = loads[read.min(loads.len() - 1)];
        read += 1;
        Ok(value)
    };
    let mut said = Vec::new();
    let outcome = wait(SETTINGS, github, &mut load, &clock, &mut |line| {
        said.push(line.to_owned());
    });
    (outcome, said, clock.elapsed.get())
}

#[test]
fn runnable_tasks_over_the_threshold_are_waited_out_saying_the_mean_every_30_s() {
    // Given: 20 at 0, 5, ... 30 s, then 11
    let loads = [20.0, 20.0, 20.0, 20.0, 20.0, 20.0, 20.0, 11.0];

    // When
    let (outcome, said, slept) = run(&loads, false);

    // Then: the mean of the last six is 11 once all six are, at 60 s
    assert_eq!(outcome, Ok(Outcome::Clear));
    assert_eq!(slept, Duration::from_secs(60));
    assert_eq!(said.len(), 2, "{said:?}");
    assert!(
        said[1].starts_with(
            "load: 20.0 runnable tasks on average over the last 6 readings, over 12 ([load] threshold); waiting, 30 s so far"
        ),
        "{said:?}"
    );
}

#[test]
fn single_low_readings_between_compile_bursts_do_not_end_the_wait() {
    // Given: lows at 5, 20 and 30 s among 30s, then lows from 35 s
    let loads = [30.0, 2.0, 30.0, 30.0, 2.0, 30.0, 2.0, 2.0, 2.0];

    // When
    let (outcome, _, slept) = run(&loads, false);

    // Then: the six readings to 40 s average 11.3, the first window under 12
    assert_eq!(outcome, Ok(Outcome::Clear));
    assert_eq!(slept, 8 * READ_EVERY);
}

#[test]
fn an_idle_machine_starts_once_the_window_is_full_saying_nothing() {
    // When
    let (outcome, said, slept) = run(&[12.0], false);

    // Then
    assert_eq!(
        (outcome, said.len(), slept),
        (Ok(Outcome::Clear), 0, (WINDOW - 1) * READ_EVERY)
    );
}

#[test]
fn at_the_deadline_it_goes_on_saying_the_load_and_the_wait() {
    // When
    let (outcome, said, slept) = run(&[30.0], false);

    // Then
    assert_eq!(outcome, Ok(Outcome::PastDeadline));
    assert_eq!(slept, SETTINGS.deadline);
    let last = said.last().expect("a line at the deadline");
    assert!(
        last.contains("STILL 30.0 runnable tasks on average"),
        "{last}"
    );
    assert!(
        last.contains("after waiting 600 s; going on anyway"),
        "{last}"
    );
}

#[test]
fn on_github_actions_it_does_not_wait_or_read_the_load() {
    // Given
    let mut load = || -> Result<f64, String> { panic!("the load was read") };

    // When
    let outcome = wait(SETTINGS, true, &mut load, &FakeClock::new(), &mut |_| {});

    // Then
    assert!(matches!(outcome, Ok(Outcome::Skipped(why)) if why.contains("single-tenant")));
}

#[test]
fn an_unreadable_load_is_refused_not_taken_as_low() {
    // Given
    let mut load = || Err("/proc/loadavg: no such file".to_owned());

    // When
    let outcome = wait(SETTINGS, false, &mut load, &FakeClock::new(), &mut |_| {});

    // Then
    assert_eq!(outcome, Err("/proc/loadavg: no such file".to_owned()));
}

#[test]
fn with_the_wait_turned_off_it_does_not_read_the_load_and_says_why() {
    // Given
    let off = Settings {
        enabled: false,
        ..SETTINGS
    };
    let mut load = || -> Result<f64, String> { panic!("the load was read") };

    // When
    let outcome = wait(off, false, &mut load, &FakeClock::new(), &mut |_| {});

    // Then
    assert_eq!(
        outcome,
        Ok(Outcome::Skipped(
            "[load] wait = false in .config/factory.toml"
        ))
    );
}

#[test]
fn an_unreadable_load_on_this_machine_names_the_switch_that_turns_the_wait_off() {
    assert!(
        NO_LOAD_FIX.contains("[load] wait = \"false\""),
        "{NO_LOAD_FIX}"
    );
}
