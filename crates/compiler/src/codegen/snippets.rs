//! Snippets (1-R1): `{snippet name(param: Type, ...)}` declares a
//! `vapors_dom::Snippet<(Type, ...)>` built against `dyn Dom`;
//! `{render name(args)}` builds one in place with its arguments; a
//! component tag's children become the child's `children` snippet, and
//! a snippet declared directly inside the tag becomes the child's prop
//! of that name.

use super::emit::{Emitter, Place};
use crate::CompileError;
use crate::ast::{Expr, Node, Render, Snippet};

impl Emitter<'_> {
    /// `{render name(args)}`: the snippet's nodes at this place, among
    /// the roots when the place is.
    pub fn emit_render(&mut self, render: &Render, place: &Place) -> Result<(), CompileError> {
        let args = if render.args.trim().is_empty() {
            "()".to_owned()
        } else {
            let expr = Expr {
                code: format!("({},)", render.args.trim()),
                offset: render.offset,
            };
            self.unwrapped(&expr)?
        };
        let call = format!(
            "{}.render(dom, {}, {}, {args})",
            render.name,
            place.parent,
            place.before()
        );
        if place.roots {
            self.line(format!("roots.extend({call});"));
        } else {
            self.line(format!("let _ = {call};"));
        }
        Ok(())
    }

    /// `{snippet name(params)}`: a local snippet. A parameter typed
    /// `Signal<..>` or `Memo<..>` reads as its value in the body, as a
    /// loop binding does.
    pub fn emit_snippet(&mut self, snippet: &Snippet) -> Result<(), CompileError> {
        let params = self.snippet_params(snippet)?;
        let names: Vec<&str> = params.iter().map(|(name, _)| name.as_str()).collect();
        let types: Vec<&str> = params.iter().map(|(_, ty)| ty.as_str()).collect();
        let (pattern, tuple) = (tuple_of(&names), tuple_of(&types));
        self.reserve(&snippet.name);
        self.line(format!(
            "let {}: Snippet<{tuple}> = Snippet::new(",
            snippet.name
        ));
        let reactive = params
            .iter()
            .filter(|(_, ty)| is_reactive(ty))
            .map(|(name, _)| name.clone());
        self.push_scope(reactive.collect());
        let built = self.emit_snippet_body(&snippet.children, &format!(", {pattern}: {tuple}"));
        self.pop_scope();
        self.line(");");
        built?;
        self.snippets.push(snippet.name.clone());
        Ok(())
    }

    /// The children of a component tag, less the snippets declared in
    /// it, as a `children` field of its props: a snippet built where the
    /// child renders it.
    pub fn emit_children(&mut self, children: &[Node], var: &str) -> Result<String, CompileError> {
        let local = format!("{var}_children");
        self.reserve(&local);
        self.line(format!("let {local} = Snippet::new("));
        let built = self.emit_snippet_body(children, ", (): ()");
        self.line(");");
        built?;
        Ok(format!("children: {local},"))
    }

    /// A snippet's closure, its body built against `dyn Dom`.
    fn emit_snippet_body(&mut self, nodes: &[Node], params: &str) -> Result<(), CompileError> {
        self.indent(1);
        let outer = std::mem::replace(&mut self.dom_type, "dyn Dom");
        let signature = self.builder_signature(params);
        let built = self.emit_builder(nodes, &signature);
        self.dom_type = outer;
        self.dedent(1);
        built
    }

    /// `(name: Type, ...)` as names and types; owned types only, since a
    /// snippet's arguments outlive the call that renders it.
    fn snippet_params(&self, snippet: &Snippet) -> Result<Vec<(String, String)>, CompileError> {
        let inner = snippet
            .params
            .trim()
            .trim_start_matches('(')
            .trim_end_matches(')');
        split_top_level(inner)
            .into_iter()
            .filter(|param| !param.trim().is_empty())
            .map(|param| self.snippet_param(snippet, param))
            .collect()
    }

    fn snippet_param(
        &self,
        snippet: &Snippet,
        param: &str,
    ) -> Result<(String, String), CompileError> {
        let refuse = |why: String| self.source.error(snippet.offset, why);
        let (name, ty) = param.split_once(':').ok_or_else(|| {
            refuse(format!(
                "snippet `{}`: parameter `{}` needs a type, `name: Type`",
                snippet.name,
                param.trim()
            ))
        })?;
        let (name, ty) = (name.trim(), ty.trim());
        if ty.contains('&') {
            return Err(refuse(format!(
                "snippet `{}`: parameter `{name}: {ty}` borrows; a snippet keeps its arguments for its effects, so give it an owned type (String, not &str)",
                snippet.name
            )));
        }
        Ok((name.to_owned(), ty.to_owned()))
    }
}

/// `(a, b,)`, or `()` for none.
fn tuple_of(items: &[&str]) -> String {
    if items.is_empty() {
        "()".to_owned()
    } else {
        format!("({},)", items.join(", "))
    }
}

fn is_reactive(ty: &str) -> bool {
    ty.starts_with("Signal<") || ty.starts_with("Memo<")
}

/// `text` split at the commas outside brackets.
fn split_top_level(text: &str) -> Vec<&str> {
    let (mut depth, mut start, mut parts) = (0_i32, 0, Vec::new());
    for (at, c) in text.char_indices() {
        match c {
            '<' | '(' | '[' => depth += 1,
            '>' | ')' | ']' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&text[start..at]);
                start = at + 1;
            }
            _ => {}
        }
    }
    parts.push(&text[start..]);
    parts
}
