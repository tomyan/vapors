//! What the check judges, written by both threads: thread ids, ticks,
//! frames and the script's phase boundaries, each with its time.

use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::ThreadId;
use std::time::Instant;

/// The script's phases, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Live off from the start.
    Off,
    /// Live switched on.
    On,
    /// The application thread blocked, inside `On`.
    Stall,
    /// Live switched off again.
    OffAgain,
    /// The script has finished.
    End,
}

#[derive(Default)]
pub struct Record {
    pub ui_thread: Option<ThreadId>,
    /// The thread each handler and toggle ran on.
    pub handlers: Vec<ThreadId>,
    /// Each tick's time and the thread its continuation ran on.
    pub ticks: Vec<(Instant, ThreadId)>,
    /// Each frame's time.
    pub frames: Vec<Instant>,
    pub phases: Vec<(Phase, Instant)>,
}

pub type Shared = Arc<Mutex<Record>>;

pub fn lock(shared: &Shared) -> MutexGuard<'_, Record> {
    shared
        .lock()
        .expect("a thread panicked while writing the record")
}

impl Record {
    /// When `phase` began, if it has.
    pub fn at(&self, phase: Phase) -> Option<Instant> {
        self.phases
            .iter()
            .find(|(p, _)| *p == phase)
            .map(|(_, at)| *at)
    }

    pub fn finished(&self) -> bool {
        self.at(Phase::End).is_some()
    }
}
