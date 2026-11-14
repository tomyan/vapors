//! Why a block or flex container is not laid out as a lane
//! (`docs/plans/splice-layout.md`, sections 4 and 10.3), so a test can
//! pin the condition it names: a declined container is laid out by
//! Taffy as before, the exact answer, slower, never a wrong one.

/// The condition that kept a container, or one of its children, out of
/// a lane, counted by 3a-2 as `UpdateCounts::lanes_declined` with the
/// first as `lane_decline`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LaneDecline {
    /// `align-content` other than `normal` or `start`: every child is
    /// shifted by the free space.
    AlignContent,
    /// A flex container: the flex lane is PR 3f.
    FlexLaneNotBuilt,
    /// A grid container: tracks are shared by several items, so an
    /// insert moves items by cell; not planned.
    Grid,
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
    /// A scroll container has a child wholly before its origin (its
    /// border box and overflow end above or left of the content box's
    /// corner, under negative margins), which Taffy leaves out of the
    /// scrollable overflow; the fold cannot leave one child out, so the
    /// container is handed to Taffy.
    BeforeScrollOrigin,
    /// A flex row (`row` or `row-reverse`): the horizontal lane is PR 6.
    FlexRow,
    /// `flex-direction: column-reverse`: positions run from the end, so
    /// every item moves when the total changes.
    ColumnReverse,
    /// `flex-wrap` other than `nowrap`: lines are formed from the
    /// running total.
    FlexWrap,
    /// `justify-content` other than `normal`, `flex-start`, `start` or
    /// `stretch`: every item is moved by the free space.
    JustifyContent,
    /// An item's `flex-grow` is above 0: free space is shared by every
    /// item.
    FlexGrow,
    /// An item's `margin-top` or `margin-bottom` is `auto`: it takes a
    /// share of the free space.
    MainAutoMargin,
    /// A percentage `row-gap` on a column whose height is not definite:
    /// Taffy resolves it against the height the items sum to.
    PercentGap,
    /// The items overflow the column's height and one of them can
    /// shrink (its `flex-shrink` is above 0 and its minimum height is
    /// below its hypothetical height), so its height depends on the
    /// total: decided at layout, from the fold.
    Shrinks,
}
