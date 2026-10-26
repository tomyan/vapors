//! Snippets (1-R1): `{snippet name(param: Type, ...)}` declares a
//! `vapors_dom::Snippet<(Type, ...)>` built against `dyn Dom`;
//! `{render name(args)}` builds one in place, its arguments a closure
//! the snippet reads them through, so it follows them as they change; a
//! component tag's children become the child's `children` snippet, and
//! a snippet declared directly inside the tag becomes the child's prop
//! of that name.

use super::emit::{Emitter, Place};
use super::snippet_list::LocalSnippet;
use super::snippet_params::Param;
use crate::CompileError;
use crate::ast::{Expr, Node, Render, Snippet};

impl Emitter<'_> {
    /// `{render name(args)}`: the snippet's nodes at this place, among
    /// the roots when the place is.
    pub fn emit_render(&mut self, render: &Render, place: &Place) -> Result<(), CompileError> {
        self.check_render(render)?;
        let args = self.emit_render_args(render)?;
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

    /// The arguments as a closure, bound before the call with the
    /// clones it needs; reading signals in it makes them the snippet's
    /// dependencies.
    fn emit_render_args(&mut self, render: &Render) -> Result<String, CompileError> {
        let Some(expr) = args_tuple(render) else {
            return Ok("|| ()".to_owned());
        };
        let tuple = self.unwrapped(&expr)?;
        let clones = self.clones_for(&[&expr])?;
        let local = self.fresh("render_args");
        self.line(format!("let {local} = {{"));
        self.indent(1);
        self.emit_clones(&clones);
        self.line(format!("move || {tuple}"));
        self.dedent(1);
        self.line("};");
        Ok(local)
    }

    /// `{snippet name(params)}`: a local snippet. Each parameter is a
    /// memo of its argument, read as its value in the body, as a loop
    /// binding is.
    pub fn emit_snippet(&mut self, snippet: &Snippet) -> Result<(), CompileError> {
        let params = self.snippet_params(snippet)?;
        let types: Vec<&str> = params.iter().map(|param| param.ty.as_str()).collect();
        let tuple = tuple_of(&types);
        self.reserve(&snippet.name);
        self.line(format!(
            "let {}: Snippet<{tuple}> = Snippet::new(",
            snippet.name
        ));
        let args = if params.is_empty() {
            "_snippet_args"
        } else {
            "snippet_args"
        };
        self.push_scope(params.iter().map(|param| param.name.clone()).collect());
        let built = self.emit_snippet_body(
            &snippet.children,
            &format!(", {args}: vapors_dom::Args<{tuple}>"),
            &param_memos(&params),
        );
        self.pop_scope();
        self.line(");");
        built?;
        self.snippets.push(LocalSnippet::of(snippet, params.len()));
        Ok(())
    }

    /// The children of a component tag, less the snippets declared in
    /// it, as a `children` field of its props: a snippet built where the
    /// child renders it.
    pub fn emit_children(&mut self, children: &[Node], var: &str) -> Result<String, CompileError> {
        let local = format!("{var}_children");
        self.reserve(&local);
        self.line(format!("let {local} = Snippet::new("));
        let built = self.emit_snippet_body(children, ", _snippet_args: vapors_dom::Args<()>", &[]);
        self.line(");");
        built?;
        Ok(format!("children: {local},"))
    }

    /// A snippet's closure, its body built against `dyn Dom`.
    fn emit_snippet_body(
        &mut self,
        nodes: &[Node],
        params: &str,
        prelude: &[String],
    ) -> Result<(), CompileError> {
        self.indent(1);
        let outer = std::mem::replace(&mut self.dom_type, "dyn Dom");
        let signature = self.builder_signature(params);
        let clones = self.builder_clones(nodes);
        let built =
            clones.and_then(|clones| self.emit_builder_with(&clones, nodes, &signature, prelude));
        self.dom_type = outer;
        self.dedent(1);
        built
    }
}

/// Each parameter as a memo of its argument, at the body's start.
fn param_memos(params: &[Param]) -> Vec<String> {
    params
        .iter()
        .enumerate()
        .map(|(at, param)| {
            format!(
                "let {} = Memo::new({{ let snippet_args = Rc::clone(&snippet_args); move || snippet_args().{at} }});",
                param.name
            )
        })
        .collect()
}

/// `(args,)` as an expression at the render, or `None` for no arguments.
fn args_tuple(render: &Render) -> Option<Expr> {
    let args = render.args.trim();
    (!args.is_empty()).then(|| Expr {
        code: format!("({args},)"),
        offset: render.offset,
    })
}

/// `(a, b,)`, or `()` for none.
fn tuple_of(items: &[&str]) -> String {
    if items.is_empty() {
        "()".to_owned()
    } else {
        format!("({},)", items.join(", "))
    }
}
