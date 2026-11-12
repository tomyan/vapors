//! How a box was asked for its layout (`LayoutBox::asked`), and what
//! it answered a size query, for a rebuilt box in its slot to be asked
//! the same and compared (`isolated.rs`).

use taffy::{Cache, CollapsibleMarginSet, LayoutInput, LayoutOutput, RunMode, Size};

/// One way a box was asked for its layout: the input, whether in its
/// parent's block formatting context (`BoxTree::lay_out` given a
/// `BlockContext`), which a block answers differently from a root, and
/// what it answered a size query.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Asked {
    pub(crate) input: LayoutInput,
    pub(crate) in_block: bool,
    /// The answer to a size query, as its parent reads it. None for a
    /// layout, whose whole answer the cache keeps, and for an answer
    /// that bubbles an out-of-flow box up to the parent.
    pub(crate) answered: Option<SizeAnswer>,
}

impl Asked {
    /// How a box was asked with `input`, and that it answered `output`.
    pub(crate) fn of(input: LayoutInput, in_block: bool, output: &LayoutOutput) -> Self {
        let answered = match input.run_mode {
            RunMode::ComputeSize => SizeAnswer::of(output),
            RunMode::PerformLayout | RunMode::PerformHiddenLayout => None,
        };
        Self {
            input,
            in_block,
            answered,
        }
    }

    /// Whether this is the way `other` was asked, whatever was answered.
    pub(crate) fn same_way(&self, other: &Self) -> bool {
        self.input == other.input && self.in_block == other.in_block
    }
}

/// What a parent reads from a size query's answer. Flexbox and grid
/// read its size (`measure_child_size`); a block container measuring
/// itself reads a child's size, margins and whether margins collapse
/// through it into its own size, and its baseline and overflow into
/// its own baseline and overflow, which only a size query above reads
/// in turn: no layout reads them. Taffy's cache keeps a size query's
/// size only, and no answer that carries margins, so the box keeps
/// these itself.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SizeAnswer {
    size: Size<f32>,
    top_margin: CollapsibleMarginSet,
    bottom_margin: CollapsibleMarginSet,
    collapses_through: bool,
}

impl SizeAnswer {
    /// What a parent reads from `output`; none when it bubbles an
    /// out-of-flow box up, which the parent places.
    pub(crate) fn of(output: &LayoutOutput) -> Option<Self> {
        if !output.oof_candidates.is_empty() {
            return None;
        }
        Some(Self {
            size: output.size,
            top_margin: output.top_margin,
            bottom_margin: output.bottom_margin,
            collapses_through: output.margins_can_collapse_through,
        })
    }
}

/// What the box before answered one way it was asked, for the rebuilt
/// box's answer to be compared with.
pub(crate) enum Answer {
    /// The whole answer to a layout, from the cache's one entry.
    Layout(LayoutOutput),
    /// What the parent read from a size query's answer, as recorded.
    Size(SizeAnswer),
}

impl Answer {
    /// What the box `asked` answered: a layout's from `cache`, a size
    /// query's as recorded; none when neither holds it (a layout's
    /// input the cache lost, or a size answer that bubbled an
    /// out-of-flow box up).
    pub(crate) fn of(asked: &Asked, cache: &mut Cache) -> Option<Self> {
        match asked.input.run_mode {
            RunMode::PerformLayout => cache.get(&asked.input).map(Self::Layout),
            RunMode::ComputeSize | RunMode::PerformHiddenLayout => asked.answered.map(Self::Size),
        }
    }

    /// Whether `new` is this answer, in what the parent reads.
    pub(crate) fn is(&self, new: &LayoutOutput) -> bool {
        match self {
            Self::Layout(old) => old == new,
            Self::Size(old) => SizeAnswer::of(new) == Some(*old),
        }
    }
}
