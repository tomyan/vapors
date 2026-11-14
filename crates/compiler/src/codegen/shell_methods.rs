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

/// `owned_by_parent`, which a parent's generated code calls on the
/// child it mounts, and the `Drop` that disposes the owner of every
/// component not so owned: its effects hold the document, so a page
/// dropped without an unmount would otherwise keep all of it (443.7 MB
/// live after a 10,000-row page was dropped, measured by
/// vapors-perf-memory, docs/perf/memory-per-element-2026-11-14.md).
/// Disposing an owner already disposed (`unmount`) does nothing.
/// While the thread is panicking it does not dispose; the reason is in
/// the generated `Drop`, where it runs.
pub(super) fn drop_impl(name: &str) -> String {
    format!(
        "
impl {name} {{
    /// Marks this component as its parent's child: its owner is a child
    /// of the parent's and is disposed with it, so dropping this struct
    /// disposes nothing. Called by the parent's generated code.
    pub fn owned_by_parent(mut self) -> Self {{
        self.disposes_on_drop = false;
        self
    }}
}}

impl Drop for {name} {{
    fn drop(&mut self) {{
        // RELAXED: not disposed while the thread is panicking: a cleanup
        // that panicked during unwinding would abort the process and lose
        // the first panic's message, so a component dropped by unwinding
        // is left undisposed (compiler-examples drop_while_panicking.rs).
        // Retired when no cleanup can panic (each refusal returned, not
        // raised, as the scene nodes' teardown does).
        if self.disposes_on_drop && !std::thread::panicking() {{
            self.owner.dispose();
        }}
    }}
}}
"
    )
}
