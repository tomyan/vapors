//! What a component type declares it dispatches, known at compile time:
//! its own `#[dispatcher(…)]` events and every child component's, so a
//! tree knows them when an ancestor mounts, before the dispatching
//! component does (an `{if}` may mount it later). `events()` on the
//! component lists them; `mount` declares them to the tree first.

use std::fmt::Write;

use crate::ast::{Component, Node};

/// The names of the child component types in the template, each once,
/// in the order first met.
pub(crate) fn child_types(component: &Component) -> Vec<String> {
    let mut found = Vec::new();
    walk(&component.template.children, &mut found);
    found
}

fn walk(nodes: &[Node], found: &mut Vec<String>) {
    for node in nodes {
        match node {
            Node::Component(tag) => {
                if !found.contains(&tag.name) {
                    found.push(tag.name.clone());
                }
                walk(&tag.children, found);
            }
            Node::Element(element) => walk(&element.children, found),
            Node::If(block) => {
                walk(&block.then, found);
                walk(block.otherwise.as_deref().unwrap_or(&[]), found);
            }
            Node::For(block) => walk(&block.children, found),
            Node::Snippet(snippet) => walk(&snippet.children, found),
            Node::Text(_) | Node::Render(_) => {}
        }
    }
}

/// `events()` and `events_into()`: the component's own event names and
/// its children's, with the script's `use` items, which bring the
/// children's types in. A type already listed is skipped, so a component
/// that uses itself, directly or through another, ends.
pub(crate) fn events_method(own: &[String], children: &[String], uses: &[&String]) -> String {
    let mut imports = String::new();
    for item in uses {
        let _ = writeln!(imports, "        #[allow(unused_imports)]\n        {item}");
    }
    let mut body = String::new();
    if !own.is_empty() {
        let own: Vec<String> = own.iter().map(|name| format!("{name:?}")).collect();
        let _ = writeln!(body, "            events.extend([{}]);", own.join(", "));
    }
    for child in children {
        let _ = writeln!(body, "            {child}::events_into(seen, events);");
    }
    // a type that dispatches nothing and has no children adds nothing
    let (into, listed) = if body.is_empty() {
        (
            "_events",
            "        seen.insert(std::any::type_name::<Self>());\n".to_owned(),
        )
    } else {
        (
            "events",
            format!("        if seen.insert(std::any::type_name::<Self>()) {{\n{body}        }}\n"),
        )
    };
    format!(
        "    /// The events this component type dispatches itself, its child
    /// components' included, which `mount` declares to the tree.
    #[must_use]
    pub fn events() -> Vec<&'static str> {{
        let mut events = Vec::new();
        Self::events_into(&mut std::collections::BTreeSet::new(), &mut events);
        events
    }}

    /// [`Self::events`] onto `events`, unless this type is in `seen`
    /// already: a component that uses itself is listed once.
    pub fn events_into(seen: &mut std::collections::BTreeSet<&'static str>, {into}: &mut Vec<&'static str>) {{
{imports}{listed}    }}
"
    )
}
