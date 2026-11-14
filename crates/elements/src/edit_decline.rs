//! Why an element's inserts and removes were not made as an edit of
//! its box children (`layout/child_edits.rs`), so a test can pin the
//! condition it names: each decline builds the element shallow.

/// The condition that built an element shallow in place of editing its
/// box children (`UpdateCounts::children_edit_decline`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EditDecline {
    /// The element makes no box of its own: `display: none`, hidden
    /// before or in this update, or removed from the document. An
    /// ancestor builds it; nothing is edited.
    OwnerHasNoBox,
    /// The element is an `svg`, a text field or a number field, whose
    /// children are not boxes.
    OwnerNotABlock,
    /// A removed child had no box: a text node with characters, an
    /// inline element or one with `display: none`.
    RemovedWithoutBox,
    /// A removed child's element is still a child of the element: it
    /// moved within it.
    Moved,
    /// A text node with characters or an inline element is beside an
    /// inserted or removed child, so a text run may change.
    TextBeside,
    /// An inserted child is a text node with characters or an inline
    /// element.
    InsertedInline,
    /// A box under the element changed in a child the edit keeps (a
    /// restyle, a text change, a child hidden or shown).
    MarkedInside,
}
