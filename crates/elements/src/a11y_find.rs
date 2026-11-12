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
    /// the role (inside `within`) and the matches outside `within`;
    /// [`AxError::Ambiguous`] when more than one does.
    pub fn ax_find(
        &self,
        role: &str,
        name: &str,
        within: Option<Named>,
    ) -> Result<AxElement, AxError> {
        let tree = self.ax_tree()?;
        let mut visits = Vec::new();
        visit(&tree, "", &[], &mut visits);
        let inside: Vec<&Visit> = visits.iter().filter(|v| is_within(v, within)).collect();
        let matches = those(&inside, |node| is(node, (role, name)));
        match matches.as_slice() {
            [one] => Ok(one.node.clone()),
            [] => Err(self.ax_unmatched(&visits, &inside, (role, name), within)),
            _ => Err(AxError::Ambiguous {
                matches: paths(&matches),
                role: role.to_owned(),
                name: name.to_owned(),
            }),
        }
    }
}

impl Document {
    /// The refusal for no match: [`AxError::Hidden`] when, with no
    /// ancestor asked, every element matching is one the tree left out;
    /// else [`AxError::Missing`].
    fn ax_unmatched(
        &self,
        all: &[Visit],
        inside: &[&Visit],
        named: Named,
        within: Option<Named>,
    ) -> AxError {
        let (role, name) = named;
        if within.is_none() && self.ax_hidden(role, name) {
            return AxError::Hidden {
                role: role.to_owned(),
                name: name.to_owned(),
            };
        }
        missing(all, inside, named)
    }

    /// Whether an element the tree leaves out has the role and name.
    fn ax_hidden(&self, role: &str, name: &str) -> bool {
        let ids = self.ax_ids();
        self.elements().into_iter().any(|node| {
            self.ax_role(node, &ids).as_deref() == Some(role)
                && self.ax_name(node, role, &ids) == name
        })
    }
}

/// Whether `visit` has an ancestor that is `within`, when given.
fn is_within(visit: &Visit, within: Option<Named>) -> bool {
    within.is_none_or(|w| visit.ancestors.iter().any(|a| is(a, w)))
}

/// The refusal for no match: the nodes of the role `inside`, and the
/// exact matches among `all` that are not inside.
fn missing(all: &[Visit], inside: &[&Visit], (role, name): Named) -> AxError {
    let of_role = those(inside, |node| node.role == role);
    let outside: Vec<&Visit> = all.iter().filter(|v| is(v.node, (role, name))).collect();
    AxError::Missing {
        role: role.to_owned(),
        name: name.to_owned(),
        candidates: paths(&of_role),
        outside: paths(&outside),
    }
}

/// The visits whose node passes `test`.
fn those<'v, 't>(
    visits: &[&'v Visit<'t>],
    test: impl Fn(&AxElement) -> bool,
) -> Vec<&'v Visit<'t>> {
    visits.iter().copied().filter(|v| test(v.node)).collect()
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
