//! The methods a parent's generated code calls on a component, the same
//! for every node component (`docs/components.md`, "Child components").

/// Implements `mount`, `events_into`, `root_entries` and `node` for
/// `$name`, a struct holding `mounted: crate::node::Mounted` and mounted
/// by its own `mount_before`.
macro_rules! node_component {
    ($name:ident, $props:ident) => {
        impl $name {
            /// Mounts the component at the end of `parent`.
            pub fn mount(
                dom: &impl vapors_dom::DynDom,
                parent: vapors_dom::NodeId,
                props: $props,
            ) -> Self {
                Self::mount_before(dom, parent, None, props)
            }

            /// Adds nothing: the component dispatches no events.
            pub fn events_into(
                seen: &mut std::collections::BTreeSet<&'static str>,
                _events: &mut Vec<&'static str>,
            ) {
                seen.insert(std::any::type_name::<Self>());
            }

            /// Its top-level entries, for a parent mounting it at its own
            /// top level.
            #[must_use]
            pub fn root_entries(&self) -> &[vapors_dom::Root] {
                &self.mounted.roots
            }

            /// The scene node it holds.
            #[must_use]
            pub fn node(&self) -> vapors_scene3d::NodeId {
                self.mounted.placement.node
            }
        }
    };
}

pub(crate) use node_component;
