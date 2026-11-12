//! `{render expr()}` where `expr` is not a snippet's plain name: any
//! expression that yields a `Renderable` (a snippet, or none), or a
//! name that reads as a signal's value. It is read in an effect, through
//! `vapors_dom::blocks::dynamic`, so the snippet shown follows it.

use super::emit::{Emitter, Place};
use super::snippets::args_tuple;
use crate::CompileError;
use crate::ast::{Expr, Render};

impl Emitter<'_> {
    /// Whether the callee is rendered by name, in place: a local
    /// snippet, a prop declared `Snippet` or `Snippet<..>`, or a plain
    /// name nothing declares (which `check_render` refuses). Anything
    /// else, an `Option<Snippet>` prop included, goes through the dynamic
    /// block, where `Renderable` takes it and rustc checks the type
    /// (review-v73).
    pub(super) fn renders_by_name(&self, render: &Render) -> bool {
        let name = &render.name;
        if self.snippets.iter().any(|snippet| &snippet.name == name) {
            return true;
        }
        let declared_snippet = self.script.props.iter().any(|prop| {
            let ty = prop.ty.trim().trim_start_matches("vapors_dom::");
            &prop.name == name && (ty == "Snippet" || ty.starts_with("Snippet<"))
        });
        // a plain name nothing declares stays here, to be refused by name
        let plain = name.chars().all(|c| c.is_alphanumeric() || c == '_');
        let known = self.unwrappable().contains(name)
            || self
                .script
                .declared
                .iter()
                .any(|(declared, _)| declared == name);
        declared_snippet || (plain && !known)
    }

    pub(super) fn emit_dynamic_render(
        &mut self,
        render: &Render,
        place: &Place,
    ) -> Result<(), CompileError> {
        let callee = Expr {
            code: render.name.clone(),
            offset: render.offset,
        };
        let read = self.unwrapped(&callee)?;
        let anchor = self.fresh("render_anchor");
        self.emit_anchor(&anchor, place);
        let handle = self.open_block_call("dynamic", place);
        self.line(format!("dom, {}, {anchor},", place.parent));
        let clones = self.clones_for(&[&callee])?;
        let borrowed = if is_parenthesised(&read) {
            format!("&{read}")
        } else {
            format!("&({read})")
        };
        let closure = format!("move || vapors_dom::Renderable::snippet({borrowed}),");
        self.emit_closure(&clones, &[closure]);
        self.emit_dynamic_args(render)?;
        self.close_block_call(handle);
        Ok(())
    }

    /// The arguments closure, read inside the snippet's effects.
    fn emit_dynamic_args(&mut self, render: &Render) -> Result<(), CompileError> {
        let Some(args) = args_tuple(render) else {
            self.line("|| (),");
            return Ok(());
        };
        let tuple = self.unwrapped(&args)?;
        let clones = self.clones_for(&[&args])?;
        self.emit_closure(&clones, &[format!("move || {tuple},")]);
        Ok(())
    }
}

/// Whether `code` is one parenthesised expression, which `&` borrows
/// as it is (clippy's `double_parens` refuses `&((..))`).
fn is_parenthesised(code: &str) -> bool {
    matches!(syn::parse_str::<syn::Expr>(code), Ok(syn::Expr::Paren(_)))
}
