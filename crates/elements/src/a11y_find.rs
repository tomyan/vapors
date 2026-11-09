//! Finding one node of the a11y tree by role and exact name (R1),
//! within an ancestor's role and name when that alone is not enough. No
//! match, or more than one, is refused naming the candidates: a driver
//! never acts on a node it did not single out. No match where a node
//! that is not drawn matches is refused as hidden.

use crate::a11y_error::AxError;
use crate::a11y_tree::AxElement;
use crate::document::Document;

/// A role and an exact name.
type Named<'a> = (&'a str, &'a str);

/// Each node of the tree in document order, with its path from the top
/// (`a "x" > b "y"`) and its ancestors.
struct Visit<'t> {
    path: String,
    ancestors: Vec<&'t AxElement>,
    node: &'t AxElement,
}

impl Document {
    /// The one node that is `role` named `name`, under an ancestor that
    /// is `within` when given.
    ///
    /// # Errors
    /// [`AxError::NotUpdated`] as [`Document::ax_tree`];
    /// [`AxError::Missing`] when no node matches, naming every node of
    /// the role; [`AxError::Ambiguous`] when more than one does.
    pub fn ax_find(
        &self,
        role: &str,
        name: &str,
        within: Option<Named>,
    ) -> Result<AxElement, AxError> {
        let tree = self.ax_tree()?;
        let mut visits = Vec::new();
        visit(&tree, "", &[], &mut visits);
        let mut matches: Vec<&Visit> = visits
            .iter()
            .filter(|v| is(v.node, (role, name)))
            .filter(|v| within.is_none_or(|w| v.ancestors.iter().any(|a| is(a, w))))
            .collect();
        let (role, name) = (role.to_owned(), name.to_owned());
        match matches.len() {
            1 => Ok(matches.remove(0).node.clone()),
            // with no ancestor asked, every element matching is one the tree left out
            0 if within.is_none() && self.ax_hidden(&role, &name) => {
                Err(AxError::Hidden { role, name })
            }
            0 => {
                let of_role: Vec<&Visit> = visits.iter().filter(|v| v.node.role == role).collect();
                Err(AxError::Missing {
                    candidates: paths(&of_role),
                    role,
                    name,
                })
            }
            _ => Err(AxError::Ambiguous {
                matches: paths(&matches),
                role,
                name,
            }),
        }
    }
}

impl Document {
    /// Whether an element the tree leaves out has the role and name.
    fn ax_hidden(&self, role: &str, name: &str) -> bool {
        let ids = self.ax_ids();
        self.elements().into_iter().any(|node| {
            self.ax_role(node, &ids).as_deref() == Some(role)
                && self.ax_name(node, role, &ids) == name
        })
    }
}

/// Whether `node` is the role and name.
fn is(node: &AxElement, (role, name): Named) -> bool {
    node.role == role && node.name == name
}

/// Each visit's path.
fn paths(visits: &[&Visit]) -> Vec<String> {
    visits.iter().map(|v| v.path.clone()).collect()
}

/// Every node under `nodes`, depth first, in document order.
fn visit<'t>(
    nodes: &'t [AxElement],
    prefix: &str,
    ancestors: &[&'t AxElement],
    out: &mut Vec<Visit<'t>>,
) {
    for node in nodes {
        let path = format!("{prefix}{} \"{}\"", node.role, node.name);
        out.push(Visit {
            path: path.clone(),
            ancestors: ancestors.to_vec(),
            node,
        });
        let mut below = ancestors.to_vec();
        below.push(node);
        visit(&node.children, &format!("{path} > "), &below, out);
    }
}
