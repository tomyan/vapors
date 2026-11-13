//! The change the Rows workload makes before each frame (`rows.rs`).

/// What changes before each frame.
#[derive(Clone, Copy, Debug)]
pub enum RowChange {
    /// Nothing: the frame updates and draws an unchanged document.
    Idle,
    /// The middle row's value, to a new five-digit text: no box moves.
    Text,
    /// `Text` on the flex-column list (`rows_list.rs`): the edited row
    /// is a flex item, asked for its size before it is laid out.
    FlexText,
    /// The middle row's `marked` class added or removed: its background.
    Class,
    /// `Class` with the pointer over the second row's button, so each
    /// update also hit-tests the pointer's position (review v201 M2).
    HoveredClass,
    /// Alternating frames: a row inserted before the middle row, then
    /// that row removed. Starts with an insert, warmup frames counted.
    Insert,
    /// The second row's height, between one and two rows: its boxes
    /// are built again and every row after it moves (IU3 3c-3e).
    Resize,
    /// The body, which scrolls its rows, scrolled down one row; back to
    /// the top at the end. Every row's boxes move.
    Scroll,
    /// `Scroll` with the middle row's label changed between a short and
    /// a long text in the same frame: the body is laid out again while
    /// it scrolls (review-v243 M2).
    ScrollSize,
}

impl RowChange {
    /// Every change, in the order the harness lists them.
    pub const ALL: [Self; 9] = [
        Self::Idle,
        Self::Text,
        Self::FlexText,
        Self::Class,
        Self::HoveredClass,
        Self::Insert,
        Self::Resize,
        Self::Scroll,
        Self::ScrollSize,
    ];

    /// The change `name` spells, as `{:?}` prints it.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|change| format!("{change:?}") == name)
    }
}

impl RowChange {
    /// The kinds of frame the change alternates between, in the order it
    /// makes them from frame 0 (warmup counted): frame `i` is of kind
    /// `i % len`. A change whose every frame does the same work has one.
    /// `--scaling` takes a median per kind, as one median over
    /// alternating kinds is the cheaper kind's slowest frame.
    #[must_use]
    pub fn frame_kinds(self) -> &'static [&'static str] {
        match self {
            Self::Insert => &["insert", "remove"],
            Self::Resize => &["taller", "shorter"],
            Self::ScrollSize => &["longer label", "shorter label"],
            Self::Idle
            | Self::Text
            | Self::FlexText
            | Self::Class
            | Self::HoveredClass
            | Self::Scroll => &["every"],
        }
    }
}
