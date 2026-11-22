//! What the check judges, written by both threads: thread ids, toggles,
//! ticks, frames, the application thread's turns and the script's phase
//! boundaries, each with its time.

use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::ThreadId;
use std::time::Instant;

/// The script's phases, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Live off from the start.
    Off,
    /// Live asked on.
    On,
    /// The application thread blocked, inside `On`.
    Stall,
    /// Live asked off again.
    OffAgain,
    /// The script has finished.
    End,
}

/// One frame: when, the tick it showed, and whether it was drawn for the
/// script (a request to toggle, or the end) rather than for a tick.
#[derive(Clone, Copy, Debug)]
pub struct Drawn {
    pub at: Instant,
    pub shown: u64,
    pub request: bool,
}

#[derive(Default)]
pub struct Record {
    pub ui_thread: Option<ThreadId>,
    /// The thread each message's handler ran on.
    pub handlers: Vec<ThreadId>,
    /// Each toggle: when, the thread it ran on, and whether Live went on.
    pub toggles: Vec<(Instant, ThreadId, bool)>,
    /// Each tick's time and the thread its continuation ran on.
    pub ticks: Vec<(Instant, ThreadId)>,
    pub frames: Vec<Drawn>,
    /// The application thread's turns, read by the window at each request
    /// frame and at the end.
    pub turns: Vec<u64>,
    /// Toggles the script has asked for and the window has not yet sent.
    pub requests: u32,
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

    /// When Live went on, then off, by the handler's toggles.
    pub fn switched(&self) -> Option<(Instant, Instant)> {
        match self.toggles.as_slice() {
            [(on, _, true), (off, _, false)] => Some((*on, *off)),
            _ => None,
        }
    }
}
