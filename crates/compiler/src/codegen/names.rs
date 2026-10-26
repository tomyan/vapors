//! Names a `.wisp` may not bind: the generated code's own parameters
//! and locals, and raw identifiers. Refusing them at the name's
//! position replaces a silent shadowing, or a compile error in `OUT_DIR`,
//! with a message in the `.wisp`.

use std::collections::HashSet;

use syn::visit::Visit;

use crate::CompileError;
use crate::source::Source;

/// Names the generated code always uses, and what for.
const RESERVED: &[(&str, &str)] = &[
    ("dom", "the Dom handle every generated call goes through"),
    ("parent", "the node the component or a block builds into"),
    ("before", "the node the component or a block builds before"),
    ("props", "the props struct mount_before receives"),
    ("owner", "the owner of the component's signals and effects"),
    ("roots", "the component's top-level nodes"),
    ("event", "the event a bind: listener receives"),
    (
        "snippet_args",
        "the arguments a snippet reads its parameters from",
    ),
];

/// Every name a pattern binds, in order.
pub(crate) fn bound_names(pat: &syn::Pat) -> Vec<String> {
    struct Names(Vec<String>);
    impl<'ast> Visit<'ast> for Names {
        fn visit_pat_ident(&mut self, pat: &'ast syn::PatIdent) {
            self.0.push(pat.ident.to_string());
            syn::visit::visit_pat_ident(self, pat);
        }
    }
    let mut names = Names(Vec::new());
    names.visit_pat(pat);
    names.0
}

/// Refuses a raw identifier or a reserved name bound at `offset`.
pub(crate) fn check_declared(
    source: &Source,
    name: &str,
    offset: usize,
) -> Result<(), CompileError> {
    if name.starts_with("r#") {
        return Err(source.error(
            offset,
            format!("`{name}` is a raw identifier, which a .wisp does not support: rename it to a plain name"),
        ));
    }
    match RESERVED.iter().find(|(reserved, _)| *reserved == name) {
        Some((_, use_)) => Err(source.error(
            offset,
            format!("`{name}` is a name the generated code uses for {use_}: rename it"),
        )),
        None => Ok(()),
    }
}

/// Refuses a declared name that the generated code also gave a node.
pub(crate) fn check_generated(
    source: &Source,
    declared: &[(String, usize)],
    generated: &HashSet<String>,
) -> Result<(), CompileError> {
    for (name, offset) in declared {
        if generated.contains(name) {
            return Err(source.error(
                *offset,
                format!("`{name}` is the name the generated code gives a node of this template: rename it"),
            ));
        }
    }
    Ok(())
}
