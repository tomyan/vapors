//! The workloads of the full run, named, each built only when run: a
//! 10,000-row page takes seconds to mount, and `--only` should not pay
//! for the pages it skips.

use crate::racer::RacerFrames;
use crate::rows::{RowChange, Rows};
use crate::run::{Settings, Workload};

/// A workload not yet built.
pub type Make = Box<dyn FnOnce() -> Box<dyn Workload>>;

/// The most frames a page of [`BIG_PAGE`] rows or more is run for, and
/// its warmup: its frames cost several times a 1,000-row page's, and
/// its counts and medians settle in far fewer.
const BIG_PAGE_SETTINGS: Settings = Settings {
    warmup: 10,
    runs: 60,
};

/// From this many rows, a workload runs at most [`BIG_PAGE_SETTINGS`].
pub const BIG_PAGE: usize = 10_000;

/// The settings `workload` runs at: `settings`, and no more frames than
/// [`BIG_PAGE_SETTINGS`] for a big page.
#[must_use]
pub fn settings_for(rows: Option<usize>, settings: Settings) -> Settings {
    if rows.is_none_or(|rows| rows < BIG_PAGE) {
        return settings;
    }
    Settings {
        warmup: settings.warmup.min(BIG_PAGE_SETTINGS.warmup),
        runs: settings.runs.min(BIG_PAGE_SETTINGS.runs),
    }
}

/// Every workload: name, its row count (none for the racer), and how to
/// build it.
#[must_use]
pub fn workloads() -> Vec<(String, (Option<usize>, Make))> {
    let mut all = Vec::new();
    for (count, change) in row_workloads() {
        let make: Make = Box::new(move || Box::new(Rows::new(count, change)));
        all.push((format!("{count} rows, {change:?}"), (Some(count), make)));
    }
    for (name, racing) in [("racer, paused", false), ("racer, racing", true)] {
        let make: Make = Box::new(move || Box::new(RacerFrames::new(racing)));
        all.push((name.into(), (None, make)));
    }
    all
}

fn row_workloads() -> Vec<(usize, RowChange)> {
    let edits = [RowChange::Idle, RowChange::Text, RowChange::Class];
    let mut all: Vec<_> = [100, 1000, 3000]
        .into_iter()
        .flat_map(|count| edits.map(|change| (count, change)))
        .collect();
    for change in [RowChange::HoveredClass, RowChange::FlexText] {
        all.extend([1000, 3000, 10_000].map(|count| (count, change)));
    }
    all.extend(
        [10_000]
            .into_iter()
            .flat_map(|count| edits.map(|change| (count, change))),
    );
    all.extend([1000, 3000, 10_000].map(|count| (count, RowChange::Insert)));
    for change in [RowChange::Resize, RowChange::Scroll, RowChange::ScrollSize] {
        all.extend([1000, 3000].map(|count| (count, change)));
    }
    all
}

#[cfg(test)]
#[path = "workloads_tests.rs"]
mod tests;
