//! The state the filter card, the chart, the search input and the orders
//! table share. Signals per value, as the components docs teach; one
//! `Copy` struct so a component takes them as one prop.

use crate::dashboard::Status;
use vapors_signals::{Signal, batch};

/// The shared signals of the overview page.
#[derive(Clone, Copy)]
pub struct Controls {
    /// The text of the search input.
    pub search: Signal<String>,
    /// Whether the chart shows the previous period too.
    pub compare: Signal<bool>,
    /// The selected chart segment.
    pub series: Signal<usize>,
    /// Whether the status `Status::ALL[k]` shows, by `k`.
    pub shown: [Signal<bool>; 4],
    /// Whether the export menu is open.
    pub menu: Signal<bool>,
}

impl Controls {
    /// The startup state: every status shown, compare off, first segment.
    pub fn new() -> Self {
        Controls {
            search: Signal::named("search", String::new()),
            compare: Signal::named("compare", false),
            series: Signal::named("series", 0),
            shown: Status::ALL.map(|status| Signal::named(status.label(), true)),
            menu: Signal::named("menu", false),
        }
    }

    /// "Reset": checks every status, turns compare off, selects the first
    /// segment and clears the search.
    pub fn reset(&self) {
        batch(|| {
            self.shown.iter().for_each(|shown| shown.set(true));
            self.compare.set(false);
            self.series.set(0);
            self.search.set(String::new());
        });
    }
}
