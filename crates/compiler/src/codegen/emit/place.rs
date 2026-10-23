//! Where the emitter places a node.

/// Where a node is placed: as a child of `parent`, and at the top
/// level of a component or block (`roots`), before `before`.
pub(crate) struct Place {
    pub parent: String,
    pub roots: bool,
}

impl Place {
    pub fn roots() -> Place {
        Place {
            parent: "parent".into(),
            roots: true,
        }
    }

    pub fn child_of(parent: &str) -> Place {
        Place {
            parent: parent.into(),
            roots: false,
        }
    }

    /// The `before` argument for a child component mount.
    pub fn before(&self) -> &'static str {
        if self.roots { "before" } else { "None" }
    }
}
