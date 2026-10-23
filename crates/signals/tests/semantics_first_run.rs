//! An effect's first run follows the same rule as its re-runs: a
//! signal it writes is picked up after the run ends, never inside it.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_signals::{Effect, Owner, Signal};

#[test]
fn an_effect_writing_a_signal_it_reads_on_its_first_run_settles() {
    // Given
    let owner = Owner::new();
    owner.set();
    let count = Signal::new(0);

    // When: each run raises the count until it reaches three
    Effect::new(move || {
        let now = count.get();
        if now < 3 {
            count.set(now + 1);
        }
    });

    // Then
    assert_eq!(count.get_untracked(), 3);
}

#[test]
fn a_write_in_a_first_run_reaches_other_effects_after_that_run_ends() {
    // Given
    let owner = Owner::new();
    owner.set();
    let b = Signal::new(0);
    let log = Rc::new(RefCell::new(Vec::new()));
    let reader_log = Rc::clone(&log);
    Effect::new(move || {
        reader_log
            .borrow_mut()
            .push(format!("reader b={}", b.get()));
    });
    let writer_log = Rc::clone(&log);

    // When
    Effect::new(move || {
        writer_log.borrow_mut().push("writer start".to_string());
        b.set(1);
        writer_log.borrow_mut().push("writer end".to_string());
    });

    // Then
    assert_eq!(
        *log.borrow(),
        ["reader b=0", "writer start", "writer end", "reader b=1"]
    );
}
