use std::cell::{Cell, RefCell};
use std::time::Duration;

use vapors_platform::Instant;

use super::*;

/// A clock a test moves by hand.
struct Stepped {
    base: Instant,
    offset: Cell<Duration>,
}

impl Stepped {
    fn new() -> Stepped {
        Stepped {
            base: Instant::now(),
            offset: Cell::new(Duration::ZERO),
        }
    }

    fn step(&self, by: Duration) {
        self.offset.set(self.offset.get() + by);
    }
}

impl Clock for Stepped {
    fn now(&self) -> Instant {
        self.base + self.offset.get()
    }
}

/// Phases that record what ran, with the frame time each saw, and
/// spend `cost` of the stepped clock in each; `fail` makes that phase
/// refuse.
struct Recorded<'c> {
    clock: &'c Stepped,
    ran: RefCell<Vec<(Phase, Instant)>>,
    fail: Option<Phase>,
}

impl Recorded<'_> {
    fn run(&mut self, phase: Phase, now: Instant) -> Result<(), String> {
        self.ran.borrow_mut().push((phase, now));
        self.clock.step(Duration::from_millis(phase as u64 + 1));
        if self.fail == Some(phase) {
            return Err(format!("{phase:?} refused"));
        }
        Ok(())
    }
}

impl Phases for Recorded<'_> {
    fn input(&mut self, now: Instant) {
        let _ = self.run(Phase::Input, now);
    }
    fn tasks(&mut self, now: Instant) {
        let _ = self.run(Phase::Tasks, now);
    }
    fn tick(&mut self, now: Instant) {
        let _ = self.run(Phase::Tick, now);
    }
    fn update(&mut self, now: Instant) -> Result<(), String> {
        self.run(Phase::Update, now)
    }
    fn accessibility(&mut self, now: Instant) {
        let _ = self.run(Phase::Accessibility, now);
    }
    fn draw(&mut self, now: Instant) -> Result<(), String> {
        self.run(Phase::Draw, now)
    }
    fn record(&mut self, now: Instant, _times: &FrameTimes) {
        let _ = self.run(Phase::Record, now);
    }
}

fn recorded(clock: &Stepped, fail: Option<Phase>) -> Recorded<'_> {
    Recorded {
        clock,
        ran: RefCell::new(Vec::new()),
        fail,
    }
}

#[test]
fn a_frame_runs_its_phases_in_order_at_one_time() {
    // Given
    let clock = Stepped::new();
    let mut phases = recorded(&clock, None);
    let now = clock.now();

    // When
    run_frame(&mut phases, now, &clock).expect("the frame runs");

    // Then: design 02's order, each phase given the frame's time
    let ran = phases.ran.into_inner();
    let order: Vec<Phase> = ran.iter().map(|(phase, _)| *phase).collect();
    let expected = [
        Phase::Input,
        Phase::Tasks,
        Phase::Tick,
        Phase::Update,
        Phase::Accessibility,
        Phase::Draw,
        Phase::Record,
    ];
    assert_eq!(order, expected);
    assert!(ran.iter().all(|(_, seen)| *seen == now));
}

#[test]
fn each_phase_is_timed_by_the_clock() {
    // Given: phase n spends n + 1 ms of the stepped clock
    let clock = Stepped::new();
    let mut phases = recorded(&clock, None);

    // When
    let times = run_frame(&mut phases, clock.now(), &clock).expect("the frame runs");

    // Then: Record is timed after the times it is given, so it is not among them
    for phase in &Phase::ALL[..6] {
        assert_eq!(
            times.of(*phase),
            Duration::from_millis(*phase as u64 + 1),
            "{phase:?}"
        );
    }
}

#[test]
fn a_failed_update_stops_the_frame_naming_the_phase() {
    // Given
    let clock = Stepped::new();
    let mut phases = recorded(&clock, Some(Phase::Update));

    // When
    let error = run_frame(&mut phases, clock.now(), &clock).expect_err("refused");

    // Then: nothing after Update ran
    assert_eq!(error.phase, Phase::Update);
    assert!(error.to_string().contains("Update refused"), "{error}");
    let last = phases.ran.into_inner().last().map(|(phase, _)| *phase);
    assert_eq!(last, Some(Phase::Update));
}

#[test]
fn a_failed_draw_stops_the_frame_before_record() {
    // Given
    let clock = Stepped::new();
    let mut phases = recorded(&clock, Some(Phase::Draw));

    // When
    let error = run_frame(&mut phases, clock.now(), &clock).expect_err("refused");

    // Then
    assert_eq!(error.phase, Phase::Draw);
    assert!(
        !phases
            .ran
            .into_inner()
            .iter()
            .any(|(phase, _)| *phase == Phase::Record)
    );
}

#[test]
fn the_input_phase_holds_the_time_handlers_took_since_the_last_frame() {
    // Given: handlers that took 5 ms as their events arrived
    struct Handled(Recorded<'static>);
    impl Phases for Handled {
        fn input_time(&mut self) -> Duration {
            Duration::from_millis(5)
        }
        fn update(&mut self, now: Instant) -> Result<(), String> {
            self.0.update(now)
        }
        fn draw(&mut self, now: Instant) -> Result<(), String> {
            self.0.draw(now)
        }
    }
    let clock: &'static Stepped = Box::leak(Box::new(Stepped::new()));
    let mut phases = Handled(recorded(clock, None));

    // When
    let times = run_frame(&mut phases, clock.now(), clock).expect("the frame runs");

    // Then: no input work in the frame itself, so the handlers' time alone
    assert_eq!(times.of(Phase::Input), Duration::from_millis(5));
}
