//! The snippets a list declares: each before the rest of the list, so
//! a `{render}` may come before its snippet, and each after the sibling
//! snippets it renders, so a snippet may render one written after it.
//! Two of one name in a list, and a snippet that renders itself, are
//! refused; a snippet is visible in its list and the lists inside it.

use std::collections::HashSet;

use super::emit::Emitter;
use crate::CompileError;
use crate::ast::{Node, Render, Snippet};

/// A snippet declared in a list being emitted, for the renders in it.
pub(crate) struct LocalSnippet {
    pub name: String,
    /// `name(params)` as written, for refusals.
    pub written: String,
    pub arity: usize,
    pub offset: usize,
}

impl LocalSnippet {
    pub(super) fn of(snippet: &Snippet, arity: usize) -> LocalSnippet {
        LocalSnippet {
            name: snippet.name.clone(),
            written: format!("{}{}", snippet.name, snippet.params.trim()),
            arity,
            offset: snippet.offset,
        }
    }
}

impl Emitter<'_> {
    /// Declares `nodes`' snippets; the caller truncates `snippets` back
    /// to its length before when the list ends.
    pub(super) fn emit_list_snippets(&mut self, nodes: &[Node]) -> Result<(), CompileError> {
        let snippets: Vec<&Snippet> = nodes
            .iter()
            .filter_map(|node| match node {
                Node::Snippet(snippet) => Some(snippet),
                _ => None,
            })
            .collect();
        self.refuse_duplicates(&snippets)?;
        let ordered = dependency_order(&snippets).map_err(|cycle| self.recursion(&cycle))?;
        for snippet in ordered {
            self.emit_snippet(snippet)?;
        }
        Ok(())
    }

    fn refuse_duplicates(&self, snippets: &[&Snippet]) -> Result<(), CompileError> {
        for (at, snippet) in snippets.iter().enumerate() {
            if let Some(first) = snippets[..at].iter().find(|s| s.name == snippet.name) {
                return Err(self.source.error(
                    snippet.offset,
                    format!(
                        "a second snippet `{}` in this list; the first is at line {}: rename one of them",
                        snippet.name,
                        self.line_of(first.offset)
                    ),
                ));
            }
        }
        Ok(())
    }

    fn recursion(&self, cycle: &[&Snippet]) -> CompileError {
        let through: Vec<String> = cycle[1..].iter().map(|s| format!("`{}`", s.name)).collect();
        let how = if through.is_empty() {
            String::new()
        } else {
            format!(" through {}", through.join(", "))
        };
        self.source.error(
            cycle[0].offset,
            format!(
                "snippet `{}` renders itself{how}; recursive snippets are not supported yet",
                cycle[0].name
            ),
        )
    }

    /// A render of a snippet in scope with as many arguments as it
    /// takes, or of a script binding or prop (whose type rustc checks).
    pub(super) fn check_render(&self, render: &Render) -> Result<(), CompileError> {
        if let Some(local) = self.snippets.iter().rev().find(|s| s.name == render.name) {
            return self.check_arity(render, local);
        }
        let script = &self.script;
        if script.bindings.contains(&render.name)
            || script.props.iter().any(|p| p.name == render.name)
        {
            return Ok(());
        }
        Err(self.source.error(
            render.offset,
            format!("`{{render {0}(..)}}`: nothing named `{0}` is declared here; declare it as a prop, `#[prop] let {0}: Snippet = Snippet::empty();`, or as a snippet in this list or one around it, `{{snippet {0}()}}...{{/snippet}}`", render.name),
        ))
    }

    fn check_arity(&self, render: &Render, local: &LocalSnippet) -> Result<(), CompileError> {
        let passed = argument_count(render).map_err(|why| self.source.error(render.offset, why))?;
        if passed == local.arity {
            return Ok(());
        }
        Err(self.source.error(
            render.offset,
            format!(
                "`{{render {}({})}}` passes {passed} arguments; `{}` (line {}) takes {}",
                render.name,
                render.args.trim(),
                local.written,
                self.line_of(local.offset),
                local.arity
            ),
        ))
    }

    /// The line of an offset, for a refusal naming another position.
    pub(super) fn line_of(&self, offset: usize) -> String {
        let described = self.source.describe(offset);
        described.split(':').next().unwrap_or("?").to_owned()
    }
}

/// The snippets with each after the siblings it renders; a cycle, its
/// snippets from the first, when there is one.
fn dependency_order<'s>(snippets: &[&'s Snippet]) -> Result<Vec<&'s Snippet>, Vec<&'s Snippet>> {
    let mut order = Vec::new();
    let mut path = Vec::new();
    for snippet in snippets {
        visit(snippet, snippets, &mut path, &mut order)?;
    }
    Ok(order)
}

fn visit<'s>(
    snippet: &'s Snippet,
    snippets: &[&'s Snippet],
    path: &mut Vec<&'s Snippet>,
    order: &mut Vec<&'s Snippet>,
) -> Result<(), Vec<&'s Snippet>> {
    if order.iter().any(|done| std::ptr::eq(*done, snippet)) {
        return Ok(());
    }
    if let Some(start) = path.iter().position(|on| std::ptr::eq(*on, snippet)) {
        return Err(path[start..].to_vec());
    }
    path.push(snippet);
    let renders = renders_in(&snippet.children);
    for sibling in snippets.iter().filter(|s| renders.contains(&s.name)) {
        visit(sibling, snippets, path, order)?;
    }
    path.pop();
    order.push(snippet);
    Ok(())
}

/// Every name a `{render}` under `nodes` renders.
fn renders_in(nodes: &[Node]) -> HashSet<String> {
    let mut names = HashSet::new();
    for node in nodes {
        match node {
            Node::Render(render) => {
                names.insert(render.name.clone());
            }
            Node::Element(element) => names.extend(renders_in(&element.children)),
            Node::Component(tag) => names.extend(renders_in(&tag.children)),
            Node::If(block) => {
                names.extend(renders_in(&block.then));
                names.extend(renders_in(block.otherwise.as_deref().unwrap_or(&[])));
            }
            Node::For(block) => names.extend(renders_in(&block.children)),
            Node::Snippet(snippet) => names.extend(renders_in(&snippet.children)),
            Node::Text(_) => {}
        }
    }
    names
}

/// How many arguments a render passes.
fn argument_count(render: &Render) -> Result<usize, String> {
    let args = render.args.trim();
    if args.is_empty() {
        return Ok(0);
    }
    syn::parse_str::<syn::ExprTuple>(&format!("({args},)"))
        .map(|tuple| tuple.elems.len())
        .map_err(|err| {
            format!(
                "`{{render {}({args})}}`: the arguments do not parse: {err}",
                render.name
            )
        })
}
