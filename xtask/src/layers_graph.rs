//! The workspace's dependency graph from `cargo metadata`, and the
//! dependencies a layering rule forbids. Normal and build dependencies
//! count, followed transitively; dev dependencies do not, since they
//! build only tests.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde_json::Value;

/// Crate names and the names each depends on.
#[derive(Debug)]
pub struct Graph {
    edges: BTreeMap<String, BTreeSet<String>>,
}

/// `crate_name` must not depend on any of `forbidden`.
#[derive(Debug)]
pub struct Rule {
    pub crate_name: String,
    pub forbidden: Vec<String>,
}

/// The graph in `metadata`, the output of `cargo metadata --format-version 1`.
/// Two versions of one crate share its name, and their dependencies are
/// merged. A field the walk needs that is missing is refused, naming it.
pub fn parse(metadata: &Value) -> Result<Graph, String> {
    let names = package_names(metadata)?;
    let nodes = metadata
        .pointer("/resolve/nodes")
        .and_then(Value::as_array)
        .ok_or("cargo metadata: no `resolve` graph")?;
    let mut edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for node in nodes {
        let (from, to) = node_edges(node, &names)?;
        edges.entry(from).or_default().extend(to);
    }
    Ok(Graph { edges })
}

fn package_names(metadata: &Value) -> Result<BTreeMap<String, String>, String> {
    let packages = metadata["packages"]
        .as_array()
        .ok_or("cargo metadata: no `packages`")?;
    packages
        .iter()
        .map(|package| {
            let id = field(package, "id", "a package")?;
            Ok((id.to_owned(), field(package, "name", id)?.to_owned()))
        })
        .collect()
}

/// A node's crate name and the names of its layering dependencies.
fn node_edges(
    node: &Value,
    names: &BTreeMap<String, String>,
) -> Result<(String, Vec<String>), String> {
    let id = field(node, "id", "a resolve node")?;
    let deps = node["deps"]
        .as_array()
        .ok_or_else(|| format!("cargo metadata: node {id} has no `deps`"))?;
    let mut to = Vec::new();
    for dep in deps {
        let pkg = field(dep, "pkg", &format!("a dependency of {id}"))?;
        if layering(dep, id)? {
            to.push(name_of(names, pkg)?);
        }
    }
    Ok((name_of(names, id)?, to))
}

fn field<'a>(value: &'a Value, key: &str, owner: &str) -> Result<&'a str, String> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("cargo metadata: {owner} has no `{key}`"))
}

fn name_of(names: &BTreeMap<String, String>, id: &str) -> Result<String, String> {
    names
        .get(id)
        .cloned()
        .ok_or_else(|| format!("cargo metadata: node {id} is not among the packages"))
}

/// A dependency that ships with the crate: normal or build, not dev.
fn layering(dep: &Value, owner: &str) -> Result<bool, String> {
    let kinds = dep["dep_kinds"]
        .as_array()
        .filter(|kinds| !kinds.is_empty())
        .ok_or_else(|| format!("cargo metadata: a dependency of {owner} has no `dep_kinds`"))?;
    Ok(kinds
        .iter()
        .any(|kind| kind["kind"].as_str() != Some("dev")))
}

/// One line per forbidden crate reached, with the path to it.
pub fn violations(graph: &Graph, rules: &[Rule]) -> Result<Vec<String>, String> {
    let mut found = Vec::new();
    for rule in rules {
        check_known(graph, rule)?;
        for forbidden in &rule.forbidden {
            if let Some(path) = path(graph, &rule.crate_name, forbidden) {
                let path = path.join(" -> ");
                found.push(format!(
                    "{} depends on {forbidden}: {path}",
                    rule.crate_name
                ));
            }
        }
    }
    Ok(found)
}

/// Every crate a rule names is in the graph.
fn check_known(graph: &Graph, rule: &Rule) -> Result<(), String> {
    let names = std::iter::once(&rule.crate_name).chain(&rule.forbidden);
    match names
        .into_iter()
        .find(|name| !graph.edges.contains_key(*name))
    {
        Some(name) => Err(format!(
            "[layers.forbid] names {name}, which is not in the dependency graph; fix: use the crate's package name"
        )),
        None => Ok(()),
    }
}

/// The shortest path from `from` to `to`, breadth first.
fn path(graph: &Graph, from: &str, to: &str) -> Option<Vec<String>> {
    let mut came_from: BTreeMap<&str, &str> = BTreeMap::new();
    let mut queue = VecDeque::from([from]);
    while let Some(at) = queue.pop_front() {
        if at == to {
            return Some(walk_back(&came_from, from, to));
        }
        for next in graph.edges.get(at).into_iter().flatten() {
            if next != from && !came_from.contains_key(next.as_str()) {
                came_from.insert(next, at);
                queue.push_back(next);
            }
        }
    }
    None
}

fn walk_back(came_from: &BTreeMap<&str, &str>, from: &str, to: &str) -> Vec<String> {
    let mut path = vec![to.to_owned()];
    let mut at = to;
    while at != from {
        at = came_from[at];
        path.push(at.to_owned());
    }
    path.reverse();
    path
}

#[cfg(test)]
#[path = "layers_graph_tests.rs"]
mod tests;
