//! The script's `use` items and the comments saying where script code
//! came from. A `use` goes to the top of the generated module, where the
//! props struct and the key statics see it too; one that names an item
//! the script declares cannot, and is refused.

use std::fmt::Write;

use super::script_uses::use_names;
use crate::CompileError;
use crate::source::Source;

/// A marker statement: prettyplease drops comments, so `format` turns
/// this line into `// from <path>:<line>` afterwards.
pub(crate) fn marker(source: &Source, site: &str, offset: usize) -> String {
    let (line, _) = source.position(offset);
    format!("const _: &str = \"{MARK}{site}:{line}\";")
}

/// What a marker's string starts with.
pub(crate) const MARK: &str = "from ";

/// Refuses a `use` whose first segment is an item the script declares.
pub(crate) fn refuse_local_uses(
    source: &Source,
    stmts: &[(syn::Stmt, usize)],
) -> Result<(), CompileError> {
    let local: Vec<String> = stmts
        .iter()
        .filter_map(|(stmt, _)| item_name(stmt))
        .collect();
    for (stmt, offset) in stmts {
        let syn::Stmt::Item(syn::Item::Use(item)) = stmt else {
            continue;
        };
        let first = use_names(item).into_iter().find_map(|name| {
            let head = name.path.first()?.clone();
            local.contains(&head).then_some(head)
        });
        if let Some(head) = first {
            return Err(source.error(*offset, format!(
                "this `use` names `{head}`, which the script declares: a `use` is emitted at the top of the generated module, where `{head}` does not exist; write the path without `use` (`{head}::..`) or move `{head}` into a module of the crate and `use crate::..` it; check with `cargo run -p vapors-compiler --bin wisp-check -- {}`",
                source.path
            )));
        }
    }
    Ok(())
}

/// The name of an item a script statement declares.
fn item_name(stmt: &syn::Stmt) -> Option<String> {
    let syn::Stmt::Item(item) = stmt else {
        return None;
    };
    let ident = match item {
        syn::Item::Struct(i) => &i.ident,
        syn::Item::Enum(i) => &i.ident,
        syn::Item::Fn(i) => &i.sig.ident,
        syn::Item::Mod(i) => &i.ident,
        syn::Item::Const(i) => &i.ident,
        syn::Item::Static(i) => &i.ident,
        syn::Item::Type(i) => &i.ident,
        syn::Item::Trait(i) => &i.ident,
        syn::Item::Union(i) => &i.ident,
        _ => return None,
    };
    Some(ident.to_string())
}

/// The imports every generated file starts with, less any name the
/// script's own `use` binds (two imports of one name would not build).
/// Dom items come through `vapors_elements::__wisp::dom`, so a page's
/// crate need not depend on `vapors-dom`.
pub(crate) fn header_imports(bound: &[&str]) -> String {
    let groups: [(&str, &[&str], bool); 4] = [
        ("std::cell", &["RefCell"], false),
        ("std::rc", &["Rc"], false),
        (
            "vapors_elements::__wisp::dom",
            &[
                "Dispatcher",
                "Dom",
                "DynDom",
                "Event",
                "NodeId",
                "PropertyValue",
                "Root",
                "SCOPE_ATTRIBUTE",
                "Snippet",
            ],
            true,
        ),
        (
            "vapors_signals",
            &["Effect", "Memo", "Owner", "ReadSignal", "Signal"],
            true,
        ),
    ];
    let mut out = String::new();
    for (path, names, allow) in groups {
        let kept: Vec<&str> = names
            .iter()
            .copied()
            .filter(|n| !bound.contains(n))
            .collect();
        if kept.is_empty() {
            continue;
        }
        if allow {
            out.push_str("#[allow(unused_imports)]\n");
        }
        let _ = writeln!(out, "use {path}::{{{}}};", kept.join(", "));
    }
    out
}
