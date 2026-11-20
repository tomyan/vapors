//! Layout comparison spike (docs/plans/layout-comparison.md): the same
//! table and deep flex trees laid out by each option, behind one trait
//! so the harness times them alike.

pub mod measure;
pub mod spec;
pub mod stats;

#[cfg(all(
    feature = "taffy",
    feature = "lanes",
    feature = "fixed",
    feature = "generated"
))]
pub mod agree;
#[cfg(feature = "lanes")]
pub mod clock;
#[cfg(feature = "fixed")]
pub mod fixed;
#[cfg(feature = "generated")]
pub mod generated;
#[cfg(feature = "lanes")]
pub mod lanes;
#[cfg(feature = "taffy")]
mod taffy_deep;
#[cfg(feature = "taffy")]
pub mod taffy_engine;

/// A border box in px, in page coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// What a tree is: a table of `rows` rows of five cells, or `depth`
/// nested flex containers (`spec.rs` says each exactly).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Workload {
    Table(usize),
    Deep(usize),
}

/// An option that cannot take a workload or a change, and why.
#[derive(Debug, PartialEq, Eq)]
pub struct Unsupported(pub String);

/// One way of laying a workload out.
pub trait Engine: Sized {
    /// The option's name in the report.
    const NAME: &'static str;

    /// The tree for `workload`, not laid out yet.
    ///
    /// # Errors
    /// The workload is outside what the option lays out.
    fn build(workload: Workload) -> Result<Self, Unsupported>;

    /// Lays the whole tree out, or what the last change left dirty.
    fn layout(&mut self);

    /// Flips one leaf between its two sizes (`spec.rs`), marking what
    /// the option needs marked.
    ///
    /// # Errors
    /// The option cannot take a size change on that leaf.
    fn change_leaf(&mut self) -> Result<(), Unsupported>;

    /// The two boxes the workload checks: `spec::probe_names`.
    fn probe(&self) -> [Rect; 2];
}
