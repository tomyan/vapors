//! A snippet's parameters, `(name: Type, ...)`, parsed as a closure's
//! are, and refused at the `{snippet}` when a name is not plain, has no
//! type, is a name the generated code or the script binds, or when a
//! type borrows or is a signal (parameters are reactive already).

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;

use super::emit::Emitter;
use super::names::check_declared;
use crate::CompileError;
use crate::ast::Snippet;

/// One parameter: its name and its type as Rust.
pub(super) struct Param {
    pub name: String,
    pub ty: String,
}

impl Emitter<'_> {
    pub(super) fn snippet_params(&self, snippet: &Snippet) -> Result<Vec<Param>, CompileError> {
        let inner = snippet.params.trim();
        let inner = inner
            .strip_prefix('(')
            .and_then(|rest| rest.strip_suffix(')'));
        let closure: syn::ExprClosure = syn::parse_str(&format!("|{}| ()", inner.unwrap_or("")))
            .map_err(|err| {
                self.source.error(
                    snippet.offset,
                    format!(
                        "snippet `{}`: the parameters `{}` are not `(name: Type, ...)`: {err}",
                        snippet.name, snippet.params
                    ),
                )
            })?;
        closure
            .inputs
            .iter()
            .map(|pat| self.snippet_param(snippet, pat))
            .collect()
    }

    fn snippet_param(&self, snippet: &Snippet, pat: &syn::Pat) -> Result<Param, CompileError> {
        let refuse = |why: String| {
            self.source
                .error(snippet.offset, format!("snippet `{}`: {why}", snippet.name))
        };
        let shown = source_of(pat);
        let syn::Pat::Type(typed) = pat else {
            return Err(refuse(format!(
                "parameter `{shown}` needs a type, `name: Type`"
            )));
        };
        let syn::Pat::Ident(ident) = &*typed.pat else {
            return Err(refuse(format!(
                "parameter `{shown}` is a pattern; give it a plain name"
            )));
        };
        let param = Param {
            name: ident.ident.to_string(),
            ty: source_of(&*typed.ty),
        };
        self.check_param_name(snippet, &param.name)?;
        check_param_type(&typed.ty, &param).map_err(refuse)?;
        Ok(param)
    }

    /// A reserved name, or one the script binds, which the parameter
    /// would shadow in some holes and not others.
    fn check_param_name(&self, snippet: &Snippet, name: &str) -> Result<(), CompileError> {
        check_declared(self.source, name, snippet.offset)?;
        let Some((_, at)) = self
            .script
            .declared
            .iter()
            .find(|(declared, _)| declared == name)
        else {
            return Ok(());
        };
        Err(self.source.error(
            snippet.offset,
            format!(
                "snippet `{}`: parameter `{name}` has the name of the script's `{name}` (line {}); rename one of them",
                snippet.name,
                self.line_of(*at)
            ),
        ))
    }
}

/// The text a node was parsed from, as written.
fn source_of(node: &(impl ToTokens + Spanned)) -> String {
    node.span()
        .source_text()
        .unwrap_or_else(|| node.to_token_stream().to_string())
}

/// A type a snippet can keep: owned, and not a signal.
fn check_param_type(ty: &syn::Type, param: &Param) -> Result<(), String> {
    let Param { name, ty: shown } = param;
    if let Some(inner) = reactive_inner(ty) {
        return Err(format!(
            "parameter `{name}: {shown}`: parameters are already reactive, and a snippet reads its arguments as they change; declare `{name}: {inner}`"
        ));
    }
    let mut borrows = Borrows(false);
    borrows.visit_type(ty);
    if borrows.0 {
        return Err(format!(
            "parameter `{name}: {shown}` borrows; a snippet keeps its arguments for its effects, so give it an owned type (String, not &str)"
        ));
    }
    Ok(())
}

/// `T` of a `Signal<T>` or `Memo<T>`.
fn reactive_inner(ty: &syn::Type) -> Option<String> {
    let syn::Type::Path(path) = ty else {
        return None;
    };
    let last = path.path.segments.last()?;
    if last.ident != "Signal" && last.ident != "Memo" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
        return None;
    };
    Some(args.args.to_token_stream().to_string())
}

/// Whether a type holds a reference other than `&'static`, outside the
/// arguments of a function type (which borrow only for the call).
struct Borrows(bool);

impl<'ast> Visit<'ast> for Borrows {
    fn visit_type_reference(&mut self, reference: &'ast syn::TypeReference) {
        let is_static = reference
            .lifetime
            .as_ref()
            .is_some_and(|l| l.ident == "static");
        self.0 |= !is_static;
        syn::visit::visit_type_reference(self, reference);
    }

    fn visit_type_fn_ptr(&mut self, function: &'ast syn::TypeFnPtr) {
        syn::visit::visit_return_type(self, &function.output);
    }

    fn visit_parenthesized_generic_arguments(
        &mut self,
        args: &'ast syn::ParenthesizedGenericArguments,
    ) {
        syn::visit::visit_return_type(self, &args.output);
    }
}
