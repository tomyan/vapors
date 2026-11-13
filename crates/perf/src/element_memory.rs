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

    /// This measurement, refused when its owners do not sum to its total,
    /// or one is below minus [`NEGATIVE_LIMIT_PERCENT`] of the total: a
    /// phase that freed that much of what another left is a charge the
    /// split cannot place, so the columns would not say where the heap is.
    ///
    /// # Errors
    /// Either, naming the owners and the total.
    pub fn checked(self) -> Result<Self, String> {
        let sum: i64 = self.owners.iter().sum();
        if sum != self.total {
            return Err(format!(
                "the owners {:?} sum to {sum}, not the total {}; fix: ElementMemory::split",
                self.owners, self.total
            ));
        }
        let floor = -(self.total.abs() * NEGATIVE_LIMIT_PERCENT / 100);
        let below = OWNERS
            .iter()
            .zip(self.owners)
            .find(|(_, bytes)| *bytes < floor);
        match below {
            Some((owner, bytes)) => Err(format!(
                "{owner} is {bytes} bytes, below {floor} ({NEGATIVE_LIMIT_PERCENT}% of the total \
                 {}): it freed what another phase left; measure that phase's structure apart",
                self.total
            )),
            None => Ok(self),
        }
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
