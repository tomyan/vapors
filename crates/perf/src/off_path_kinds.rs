//! The off-path pages, each with the changes that apply to it, and the
//! workloads `--off-path` runs: every kind at every size with each of
//! its changes.

use crate::off_path::{OffPath, OffPathChange};
use crate::off_path_page::Shape;
use crate::off_path_scrollers::{CODE_BLOCKS, NESTED_SCROLLERS};
use crate::workloads::Make;

use OffPathChange::{Edit, Insert, Scroll, ScrollInner};

/// The unit counts every kind runs at.
pub const SIZES: [usize; 3] = [1000, 3000, 10_000];

/// An off-path page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A scroller of rows, each row a horizontal scroller of cells.
    NestedScrollers,
    /// A transcript of messages, each with a small scrolling code block.
    CodeBlocks,
}

impl Kind {
    /// Every kind, in the order the run lists them.
    pub const ALL: [Self; 2] = [Self::NestedScrollers, Self::CodeBlocks];

    /// What the page's units are called in a workload's name. None has
    /// "rows" in it, so `--only "1000 rows"` keeps meaning the rows page.
    #[must_use]
    pub fn noun(self) -> &'static str {
        match self {
            Self::NestedScrollers => "nested scrollers",
            Self::CodeBlocks => "code blocks",
        }
    }

    /// The changes this kind runs with.
    #[must_use]
    pub fn changes(self) -> &'static [OffPathChange] {
        match self {
            Self::NestedScrollers | Self::CodeBlocks => &[Edit, Scroll, ScrollInner, Insert],
        }
    }

    /// The page's units and styles.
    #[must_use]
    pub fn shape(self) -> Shape {
        match self {
            Self::NestedScrollers => NESTED_SCROLLERS,
            Self::CodeBlocks => CODE_BLOCKS,
        }
    }
}

/// Every off-path workload: name, unit count, and how to build it.
#[must_use]
pub fn off_path_workloads() -> Vec<(String, (Option<usize>, Make))> {
    let mut all = Vec::new();
    for kind in Kind::ALL {
        for count in SIZES {
            for &change in kind.changes() {
                let make: Make = Box::new(move || Box::new(OffPath::new(kind, count, change)));
                let name = format!("{count} {}, {change:?}", kind.noun());
                all.push((name, (Some(count), make)));
            }
        }
    }
    all
}

#[cfg(test)]
#[path = "off_path_kinds_tests.rs"]
mod tests;
