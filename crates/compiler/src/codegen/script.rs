//! What the `<script>` declares: signals and memos (struct fields),
//! `#[prop]` bindings (the props struct), closures (handlers and their
//! arity), and the statements to run at mount, in order.

use std::collections::HashMap;

use proc_macro2::LineColumn;
use syn::spanned::Spanned;

use super::names::{bound_names, check_declared};
use super::script_syntax::{binding, call_of_new, named, reactive_kind};
use super::syn_map::{offset_in, render};
use crate::CompileError;
use crate::ast::Section;
use crate::source::Source;

/// A signal or memo field of the component struct.
pub(crate) struct Field {
    pub name: String,
    pub ty: String,
}

/// A `#[prop] let name: Type = default;`.
pub(crate) struct Prop {
    pub name: String,
    pub ty: String,
    pub default: String,
}

#[derive(Default)]
pub(crate) struct ScriptInfo {
    pub fields: Vec<Field>,
    pub props: Vec<Prop>,
    /// Closure bindings and how many parameters each takes.
    pub handlers: HashMap<String, usize>,
    /// Names that read as `name.get()` in text holes.
    pub unwrappable: Vec<String>,
    /// Other bindings: cloned into each closure that reads them.
    pub bindings: Vec<String>,
    /// The statements to run at mount, rendered.
    pub statements: Vec<String>,
    /// Every name the script binds, with its position.
    pub declared: Vec<(String, usize)>,
}

/// Parses and classifies the script.
pub(crate) fn analyse(
    source: &Source,
    script: Option<&Section>,
) -> Result<ScriptInfo, CompileError> {
    let Some(section) = script else {
        return Ok(ScriptInfo::default());
    };
    let block =
        syn::parse_str::<syn::Block>(&format!("{{\n{}\n}}", section.text)).map_err(|err| {
            source.error(
                position(section, err.span().start()),
                format!("in <script>: {err}"),
            )
        })?;
    let mut info = ScriptInfo::default();
    for stmt in block.stmts {
        classify(source, section, stmt, &mut info)?;
    }
    Ok(info)
}

/// Maps a position in the wrapped block (`{\n` + text + `\n}`) to the file.
fn position(section: &Section, at: LineColumn) -> usize {
    offset_in(
        &section.text,
        section.offset,
        LineColumn {
            line: at.line.saturating_sub(1).max(1),
            column: at.column,
        },
    )
}

fn classify(
    source: &Source,
    section: &Section,
    stmt: syn::Stmt,
    info: &mut ScriptInfo,
) -> Result<(), CompileError> {
    let syn::Stmt::Local(local) = stmt else {
        info.statements.push(render(&stmt));
        return Ok(());
    };
    let offset = position(section, local.span().start());
    for name in bound_names(&local.pat) {
        check_declared(source, &name, offset)?;
        info.declared.push((name, offset));
    }
    let is_prop = local.attrs.iter().any(|attr| attr.path().is_ident("prop"));
    let (name, ty) = binding(&local.pat);
    let init = local.init.as_ref().map(|init| (*init.expr).clone());
    match (is_prop, name, ty) {
        (true, Some(name), ty) => classify_prop(source, offset, &name, ty, init, info),
        (true, None, _) => Err(source.error(offset, PROP_NEEDS_A_NAME)),
        (false, Some(name), Some(ty)) if reactive_kind(&ty).is_some() => {
            classify_field(source, offset, &name, &ty, init, info)
        }
        (false, Some(name), None) if init.as_ref().and_then(call_of_new).is_some() => {
            let kind = init.as_ref().and_then(call_of_new).unwrap_or("Signal");
            Err(source.error(offset, needs_annotation(&name, kind)))
        }
        (false, name, _) => {
            classify_plain(name, init.as_ref(), info);
            info.statements.push(render(&syn::Stmt::Local(local)));
            Ok(())
        }
    }
}

const PROP_NEEDS_A_NAME: &str = "#[prop] needs a single name, which becomes its field in the props struct: `#[prop] let name: T = default;`";

fn needs_annotation(name: &str, kind: &str) -> String {
    format!(
        "`{name}` needs a type annotation to become a field of the component: write `let {name}: {kind}<T> = {kind}::new(...)`"
    )
}

/// A plain binding: cloned into closures; a closure is also a handler.
fn classify_plain(name: Option<String>, init: Option<&syn::Expr>, info: &mut ScriptInfo) {
    let Some(name) = name else {
        return;
    };
    if let Some(syn::Expr::Closure(closure)) = init {
        info.handlers.insert(name.clone(), closure.inputs.len());
    }
    info.bindings.push(name);
}

fn classify_prop(
    source: &Source,
    offset: usize,
    name: &str,
    ty: Option<syn::Type>,
    init: Option<syn::Expr>,
    info: &mut ScriptInfo,
) -> Result<(), CompileError> {
    let Some(ty) = ty else {
        return Err(source.error(
            offset,
            format!("prop `{name}` needs a type, which becomes its field in the props struct: `#[prop] let {name}: T = default;`"),
        ));
    };
    let Some(default) = init else {
        return Err(source.error(
            offset,
            format!("prop `{name}` needs a default value, used when the parent does not pass one: `#[prop] let {name}: T = default;`"),
        ));
    };
    if reactive_kind(&ty).is_some() {
        info.unwrappable.push(name.to_string());
    } else {
        info.bindings.push(name.to_string());
    }
    info.props.push(Prop {
        name: name.to_string(),
        ty: render(&ty),
        default: render(&default),
    });
    Ok(())
}

fn classify_field(
    source: &Source,
    offset: usize,
    name: &str,
    ty: &syn::Type,
    init: Option<syn::Expr>,
    info: &mut ScriptInfo,
) -> Result<(), CompileError> {
    let Some(init) = init else {
        return Err(source.error(
            offset,
            format!("`{name}` needs an initial value: `= Signal::new(...)`"),
        ));
    };
    let init = named(init, name);
    info.unwrappable.push(name.to_string());
    info.fields.push(Field {
        name: name.to_string(),
        ty: render(ty),
    });
    let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
    let stmt: syn::Stmt = syn::parse_quote!(let #ident: #ty = #init;);
    info.statements.push(render(&stmt));
    Ok(())
}
