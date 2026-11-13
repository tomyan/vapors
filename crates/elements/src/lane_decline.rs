//! Why a block container is not laid out as a lane
//! (`docs/plans/splice-layout.md`, sections 4 and 10.3), so a test can
//! pin the condition it names: a declined container is laid out by
//! Taffy as before, the exact answer, slower, never a wrong one.

/// The condition that kept a container, or one of its children, out of
/// a lane (`UpdateCounts::lane_decline`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LaneDecline {
    /// A child's margins collapse through it (empty, no padding or
    /// border): the join in the fold is PR 3b's.
    CollapsesThrough,
    /// The container was asked without a known width, so its width
    /// is its widest child's: the max-content fold is a follow-up.
    WidthUnknown,
    /// A child is `position: absolute` or `fixed`: its static position
    /// is its in-flow place among the siblings, read from the tree in
    /// PR 3b.
    OutOfFlowChild,
    /// A child bubbles an absolute or fixed box up to the container,
    /// which places it from the child's position (PR 3b).
    HoistedThrough,
    /// A child's `justify-self` is neither `normal` nor `stretch` and
    /// its width is `auto`, so its width is measured as `fit-content`:
    /// not mirrored.
    JustifySelfFitContent,
    /// A child's width or height is a sizing keyword (`min-content`,
    /// `max-content`, `fit-content`, `stretch`), which vapors' styles
    /// never produce: not mirrored.
    SizingKeyword,
}
