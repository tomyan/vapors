//! The judgement of `--check`: every rule the record breaks, each naming
//! its fix. Empty when the run is right.

use std::thread::ThreadId;

use crate::record::Record;

/// Every broken rule, given the application thread's id.
pub fn problems(record: &Record, app: ThreadId) -> Vec<String> {
    let mut found = threads(record, app);
    let Some(window) = Window::of(record) else {
        found.push(format!(
            "the run is incomplete (phases {:?}, toggles {}); fix: the script or the toggle path",
            record
                .phases
                .iter()
                .map(|(phase, _)| phase)
                .collect::<Vec<_>>(),
            record.toggles.len()
        ));
        return found;
    };
    found.extend(crate::check_idle::problems(record, &window));
    found.extend(crate::check_ticks::problems(record, &window));
    found
}

/// The run's boundaries: the script's marks and the handler's toggles.
pub struct Window {
    pub off: std::time::Instant,
    pub stall: std::time::Instant,
    /// When the handler switched Live on, then off.
    pub on_at: std::time::Instant,
    pub off_at: std::time::Instant,
}

impl Window {
    fn of(record: &Record) -> Option<Window> {
        use crate::record::Phase;
        let (on_at, off_at) = record.switched()?;
        record.at(Phase::End)?;
        Some(Window {
            off: record.at(Phase::Off)?,
            stall: record.at(Phase::Stall)?,
            on_at,
            off_at,
        })
    }
}

fn threads(record: &Record, app: ThreadId) -> Vec<String> {
    let mut found = Vec::new();
    match record.ui_thread {
        None => found.push("no frame recorded the UI thread; fix: the window's frame".to_owned()),
        Some(ui) if ui == app => found
            .push("the UI and the application thread are one; fix: AppThread::start".to_owned()),
        Some(_) => {}
    }
    let toggles = record.toggles.iter().map(|(_, id, _)| *id);
    let ticks = record.ticks.iter().map(|(_, id)| *id);
    for (what, ids, least) in [
        ("handlers", record.handlers.clone(), 3),
        ("toggles", toggles.collect(), 2),
        ("tick continuations", ticks.collect(), 1),
    ] {
        found.extend(off_thread(what, &ids, least, app));
    }
    found
}

/// A problem when fewer than `least` ran, or any ran off `app`.
fn off_thread(what: &str, ids: &[ThreadId], least: usize, app: ThreadId) -> Option<String> {
    let off = ids.iter().filter(|id| **id != app).count();
    (ids.len() < least || off > 0).then(|| {
        format!(
            "{off} of {} {what} ran off the application thread, at least {least} expected; \
             fix: send them through AppThread",
            ids.len()
        )
    })
}

/// How to run the check again.
pub const REPRODUCE: &str =
    "reproduce: xvfb-run -a cargo run -p vapors-app --example live_timer -- --check";

/// Each tick's time to the first frame showing it, in tick order.
pub fn latencies(record: &Record) -> Vec<std::time::Duration> {
    record
        .ticks
        .iter()
        .zip(1_u64..)
        .filter_map(|((tick, _), shown)| {
            let frame = record.frames.iter().find(|frame| frame.shown >= shown)?;
            Some(frame.at.saturating_duration_since(*tick))
        })
        .collect()
}
