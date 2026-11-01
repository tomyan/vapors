//! A script's `#[dispatcher(names…)] let events;`: the component's own
//! events (doc 03, "A component's own events").

use super::script::ScriptInfo;
use super::script_syntax::binding;
use crate::CompileError;
use crate::source::Source;

/// `#[dispatcher(a, b)] let events;`: a `Dispatcher` for the events
/// named, declared to the tree at mount, and a binding handlers clone.
pub(super) fn classify_dispatcher(
    source: &Source,
    offset: usize,
    attr: &syn::Attribute,
    local: &syn::Local,
    info: &mut ScriptInfo,
) -> Result<(), CompileError> {
    let refuse = |why: &str| {
        source.error(
            offset,
            format!("{why}: write `#[dispatcher(eventname, …)] let events;`"),
        )
    };
    let (Some(name), None) = binding(&local.pat) else {
        return Err(refuse("#[dispatcher] binds a single name, with no type"));
    };
    if local.init.is_some() {
        return Err(refuse(
            "#[dispatcher] takes no initialiser; the compiler makes the Dispatcher",
        ));
    }
    let names = attr
        .parse_args_with(
            syn::punctuated::Punctuated::<syn::Ident, syn::Token![,]>::parse_terminated,
        )
        .ok()
        .filter(|names| !names.is_empty())
        .ok_or_else(|| {
            refuse("#[dispatcher(…)] names the events the component dispatches, as identifiers")
        })?;
    let quoted: Vec<String> = names
        .iter()
        .map(|n| format!("{:?}", n.to_string()))
        .collect();
    info.statements.push(format!(
        "let {name} = Dispatcher::new(dom.dyn_dom(), &[{}]);",
        quoted.join(", ")
    ));
    info.bindings.push(name);
    Ok(())
}
