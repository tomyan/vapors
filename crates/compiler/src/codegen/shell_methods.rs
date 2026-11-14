//! A component's methods after its mount: the root accessors, `unmount`,
//! and the `Drop` that disposes a top-level component's owner.

/// The roots' accessors and `unmount`, closing the `impl`.
pub(super) fn accessors() -> String {
    "
    /// The top-level nodes now, in order, including those of a block
    /// or child component at the top level.
    #[must_use]
    pub fn roots(&self) -> Vec<NodeId> {
        vapors_dom::current_nodes(&self.roots)
    }

    /// The top-level entries, for a parent that mounts this component
    /// at its own top level.
    #[must_use]
    pub fn root_entries(&self) -> &[Root] {
        &self.roots
    }

    /// Reads the top-level nodes, disposes the signals and effects, and
    /// removes the nodes.
    pub fn unmount<D: Dom + ?Sized>(self, dom: &Rc<RefCell<D>>) {
        let nodes = self.roots();
        self.owner.dispose();
        for node in nodes {
            dom.borrow_mut().remove(node);
        }
    }
}
"
    .to_string()
}

/// Dropping a component mounted with no current owner disposes its
/// owner: its effects hold the document, so a page dropped without an
/// unmount would otherwise keep all of it (a 10,000-row page, 979 MB).
/// Disposing an owner already disposed (`unmount`) does nothing.
pub(super) fn drop_impl(name: &str) -> String {
    format!(
        "
impl Drop for {name} {{
    fn drop(&mut self) {{
        if self.disposes_on_drop {{
            self.owner.dispose();
        }}
    }}
}}
"
    )
}
