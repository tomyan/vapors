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
pub fn parse(metadata: &Value) -> Result<Graph, String> {
    let names: BTreeMap<&str, &str> = metadata["packages"]
        .as_array()
        .ok_or("cargo metadata: no `packages`")?
        .iter()
        .filter_map(|p| Some((p["id"].as_str()?, p["name"].as_str()?)))
        .collect();
    let nodes = metadata
        .pointer("/resolve/nodes")
        .and_then(Value::as_array)
        .ok_or("cargo metadata: no `resolve` graph")?;
    let name = |id: &str| names.get(id).map(|n| (*n).to_owned());
    let mut edges = BTreeMap::new();
    for node in nodes {
        let Some(from) = node["id"].as_str().and_then(name) else {
            continue;
        };
        let deps = node["deps"].as_array().into_iter().flatten();
        let to = deps
            .filter(|d| layering(d))
            .filter_map(|d| d["pkg"].as_str().and_then(name));
        edges.insert(from, to.collect());
    }
    Ok(Graph { edges })
}

/// A dependency that ships with the crate: normal or build, not dev.
fn layering(dep: &Value) -> bool {
    dep["dep_kinds"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|kind| kind["kind"].as_str() != Some("dev"))
}

/// One line per forbidden crate reached, with the path to it.
pub fn violations(graph: &Graph, rules: &[Rule]) -> Result<Vec<String>, String> {
    let mut found = Vec::new();
    for rule in rules {
        for name in std::iter::once(&rule.crate_name).chain(&rule.forbidden) {
            if !graph.edges.contains_key(name) {
                return Err(format!(
                    "[layers.forbid] names {name}, which is not in the dependency graph; fix: use the crate's package name"
                ));
            }
        }
        for forbidden in &rule.forbidden {
            if let Some(path) = path(graph, &rule.crate_name, forbidden) {
                found.push(format!(
                    "{} depends on {forbidden}: {}",
                    rule.crate_name,
                    path.join(" -> ")
                ));
            }
        }
    }
    Ok(found)
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
