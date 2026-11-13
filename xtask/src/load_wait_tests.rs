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
fn runnable_tasks_over_the_threshold_are_waited_out_saying_so_every_30_s() {
    // Given: over at 0, 5, ... 30 s, under at 35 s
    let loads = [20.0, 20.0, 20.0, 20.0, 20.0, 20.0, 15.0, 11.0];

    // When
    let (outcome, said, slept) = run(&loads, false);

    // Then
    assert_eq!(outcome, Ok(Outcome::Clear));
    assert_eq!(slept, Duration::from_secs(35));
    assert_eq!(said.len(), 2, "{said:?}");
    assert!(
        said[1].starts_with(
            "load: 15 runnable tasks, over 12 ([load] threshold); waiting, 30 s so far"
        ),
        "{said:?}"
    );
}

#[test]
fn the_wait_reads_again_every_5_s_and_starts_at_the_first_reading_under() {
    // When
    let (outcome, said, slept) = run(&[13.0, 12.0], false);

    // Then
    assert_eq!(outcome, Ok(Outcome::Clear));
    assert_eq!(slept, READ_EVERY);
    assert_eq!(said.len(), 1, "{said:?}");
}

#[test]
fn a_load_at_the_threshold_starts_at_once_saying_nothing() {
    // When
    let (outcome, said, slept) = run(&[12.0], false);

    // Then
    assert_eq!(
        (outcome, said.len(), slept),
        (Ok(Outcome::Clear), 0, Duration::ZERO)
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
    assert!(last.contains("STILL 30 runnable tasks"), "{last}");
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
