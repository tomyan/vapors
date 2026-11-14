//! A page's live heap after its mount and settle, split by the phase
//! that was open when it changed (finish-line property 8, memory per
//! element), and the one line a measuring child prints for it.
//!
//! The bytes are requested sizes (`Layout::size`), not malloc's overhead
//! and not GPU memory, and they are what is live after the settle, not
//! the peak. An owner's bytes are the net change while its phase was
//! open: bytes allocated in one phase and freed in another are charged
//! plus to the first and minus to the second, so an owner says where
//! the heap changed, not which structure holds it (the review of #320,
//! M1). The owners sum to the total exactly.

use std::collections::BTreeMap;

/// Prefix of the line a child prints.
const PREFIX: &str = "element-memory";

/// The owners the live bytes are split between, in line order: the
/// mount (outside any frame), the update's phases (the `layout` span
/// also covers stackings and `painted_from`, paint's preparation, so
/// `paint` is paint's own pass alone), the update's own work outside
/// them, the accessibility and draw phases, and the rest: the other
/// frame phases (input, tasks, tick, record) and everything outside a
/// frame phase after the mount.
pub const OWNERS: [&str; 8] = [
    "mount",
    "style",
    "layout",
    "paint",
    "update_rest",
    "accessibility",
    "draw",
    "rest",
];

/// The most an owner may be below zero, as a share of the total: the
/// first readings had the update's rest at -0.4% of the total at 10,000
/// rows (#321), the most of any size.
pub const NEGATIVE_LIMIT_PERCENT: i64 = 1;

/// The most the owners no named phase holds (the update's rest and the
/// rest) may hold, as a share of the total: the readings had both
/// below zero at every size, the update's rest at -0.37% of the total
/// at 10,000 rows the furthest (#321).
pub const UNPLACED_LIMIT_PERCENT: i64 = 1;

/// The owners no named phase holds.
const UNPLACED: [&str; 2] = ["update_rest", "rest"];

/// The fields of a line after its prefix, in order.
const COUNTS: [&str; 4] = ["rows", "elements", "styles_computed", "total"];

/// One page's measurement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElementMemory {
    /// The page's row count.
    pub rows: usize,
    /// Elements the first update styled: every element of the page.
    pub elements: usize,
    /// Styles the first update computed (one per distinct shareable
    /// style key; property 8's "one style record per distinct style").
    pub styles_computed: usize,
    /// Live heap bytes after the settle less before the mount.
    pub total: i64,
    /// [`OWNERS`]' net live bytes, in that order.
    pub owners: [i64; 8],
}

impl ElementMemory {
    /// The owners' bytes from the mount's bytes, the frames' phase
    /// totals (`HeapPhases`), and the page's `total`; they sum to it.
    #[must_use]
    pub fn split(mount: i64, phases: &BTreeMap<String, i64>, total: i64) -> [i64; 8] {
        let phase = |name: &str| phases.get(name).copied().unwrap_or(0);
        let update_phases = phase("style") + phase("layout") + phase("paint");
        let named = phase("update") + phase("accessibility") + phase("draw");
        [
            mount,
            phase("style"),
            phase("layout"),
            phase("paint"),
            phase("update") - update_phases,
            phase("accessibility"),
            phase("draw"),
            total - mount - named,
        ]
    }

    /// This measurement, refused when its total is negative, its owners
    /// do not sum to it, one is below minus [`NEGATIVE_LIMIT_PERCENT`] of
    /// it (a phase that freed that much of what another left is a charge
    /// the split cannot place), or the update's rest or the rest holds
    /// over [`UNPLACED_LIMIT_PERCENT`] of it (bytes no named phase owns).
    ///
    /// # Errors
    /// The first of these, naming the owners and the total.
    pub fn checked(self) -> Result<Self, String> {
        self.total_not_negative()?;
        self.owners_sum_to_total()?;
        self.owners_within_limits()?;
        Ok(self)
    }

    fn total_not_negative(&self) -> Result<(), String> {
        if self.total >= 0 {
            return Ok(());
        }
        Err(format!(
            "the total is {} bytes: a page leaves the heap smaller than before its mount, so \
             its live count is wrong; fix: read live bytes before and after the page alone",
            self.total
        ))
    }

    fn owners_sum_to_total(&self) -> Result<(), String> {
        let sum: i64 = self.owners.iter().sum();
        if sum == self.total {
            return Ok(());
        }
        Err(format!(
            "the owners {:?} sum to {sum}, not the total {}; fix: ElementMemory::split",
            self.owners, self.total
        ))
    }

    fn owners_within_limits(&self) -> Result<(), String> {
        let share = |percent: i64| self.total * percent / 100;
        let (floor, unplaced) = (
            -share(NEGATIVE_LIMIT_PERCENT),
            share(UNPLACED_LIMIT_PERCENT),
        );
        for (owner, bytes) in OWNERS.iter().zip(self.owners) {
            if bytes < floor {
                return Err(format!(
                    "{owner} is {bytes} bytes, below {floor} ({NEGATIVE_LIMIT_PERCENT}% of the total \
                     {}): it freed what another phase left; measure that phase's structure apart",
                    self.total
                ));
            }
            if UNPLACED.contains(owner) && bytes > unplaced {
                return Err(format!(
                    "{owner} holds {bytes} bytes, over {unplaced} ({UNPLACED_LIMIT_PERCENT}% of the \
                     total {}): bytes no named phase owns; meter the work that left them",
                    self.total
                ));
            }
        }
        Ok(())
    }

    /// The line a child prints for this measurement.
    #[must_use]
    pub fn line(&self) -> String {
        let counts = [
            self.rows.to_string(),
            self.elements.to_string(),
            self.styles_computed.to_string(),
            self.total.to_string(),
        ];
        let owners = self.owners.map(|bytes| bytes.to_string());
        let fields = COUNTS.iter().zip(counts).chain(OWNERS.iter().zip(owners));
        let pairs: Vec<String> = fields
            .map(|(name, value)| format!("{name}={value}"))
            .collect();
        format!("{PREFIX} {}", pairs.join(" "))
    }
}

/// The measurement a line spells.
///
/// # Errors
/// A line that is not [`ElementMemory::line`]'s shape, naming the line.
pub fn parse_line(line: &str) -> Result<ElementMemory, String> {
    let refuse = || format!("not an {PREFIX} line of {COUNTS:?} then {OWNERS:?}: {line:?}");
    let rest = line.strip_prefix(PREFIX).ok_or_else(refuse)?;
    let names = COUNTS.iter().chain(OWNERS.iter());
    let values: Option<Vec<i64>> = rest
        .split_whitespace()
        .zip(names.clone())
        .map(|(pair, name)| pair.strip_prefix(name)?.strip_prefix('=')?.parse().ok())
        .collect();
    let values =
        values.filter(|v| v.len() == names.count() && rest.split_whitespace().count() == v.len());
    let values = values.ok_or_else(refuse)?;
    let count = |at: usize| usize::try_from(values[at]).map_err(|_| refuse());
    Ok(ElementMemory {
        rows: count(0)?,
        elements: count(1)?,
        styles_computed: count(2)?,
        total: values[3],
        owners: values[4..].try_into().map_err(|_| refuse())?,
    })
}

#[cfg(test)]
#[path = "element_memory_tests.rs"]
mod tests;
