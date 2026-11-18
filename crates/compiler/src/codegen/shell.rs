//! The file around the template code: imports, the style constant,
//! the props and component structs, and the mount and unmount methods.

use std::fmt::Write;

use super::allowances::mount_allowances;
use super::script::ScriptInfo;
use crate::ast::Component;

/// The whole generated file, before prettyplease lays it out.
pub(crate) fn assemble(
    component: &Component,
    info: &ScriptInfo,
    body: &str,
    keys: &[String],
) -> String {
    let name = &component.name;
    let style = component
        .style
        .as_ref()
        .map_or("", |section| section.text.as_str());
    let mut file = header(component.site_name(), style, (&component.name, info));
    let scope = component.style.as_ref().and(component.scope.as_ref());
    if let Some(scope) = scope {
        file.push_str(&super::shell_sheet::scope_constant(scope));
    }
    let global = component.style.is_some() && scope.is_none();
    if global {
        file.push_str(&super::shell_sheet::sheet_constant(
            component.global_sheet_name(),
        ));
    }
    file.push_str(&super::style_key::key_statics(keys));
    file.push_str(&props_struct(name, info));
    file.push_str(&component_struct(name, info));
    let registration = super::shell_sheet::registration(scope.is_some(), global, style);
    file.push_str(&mount_methods(name, info, body, &registration));
    file.push_str(&super::events_decl::events_method(
        &info.events,
        &super::events_decl::child_types(component),
    ));
    file.push_str(&super::shell_methods::accessors());
    file.push_str(&super::shell_methods::drop_impl(name));
    file
}

fn header(path: &str, style: &str, (name, info): (&str, &ScriptInfo)) -> String {
    let bound: Vec<&str> = info.uses.iter().map(|used| used.name.as_str()).collect();
    let mut out = super::script_hoist::header_imports(&bound);
    let own = [name.to_owned(), format!("{name}Props")];
    for (item, names) in &info.use_items {
        // a component that uses itself: the module defines the names
        if names.iter().all(|used| own.contains(used)) {
            continue;
        }
        out.push_str(item);
        out.push('\n');
    }
    format!(
        "{out}
/// The `<style>` of `{path}`, resolved at run time by the css crate.
pub const STYLE: &str = {style:?};

"
    )
}

fn props_struct(name: &str, info: &ScriptInfo) -> String {
    let doc = format!("/// What a parent passes to [`{name}::mount`]; each prop has a default.\n");
    if info.props.is_empty() {
        return format!("{doc}#[derive(Default)]\npub struct {name}Props {{}}\n");
    }
    let mut fields = String::new();
    let mut defaults = String::new();
    for prop in &info.props {
        let _ = writeln!(
            fields,
            "    /// `{0}`, declared with `#[prop]` at `{2}`.\n    pub {0}: {1},",
            prop.name, prop.ty, prop.at
        );
        let _ = writeln!(defaults, "            {}: {},", prop.name, prop.default);
    }
    format!(
        "{doc}pub struct {name}Props {{
{fields}}}

#[allow(clippy::derivable_impls, reason = \"{DEFAULTS_AS_WRITTEN}\")]
impl Default for {name}Props {{
    fn default() -> Self {{
        Self {{
{defaults}        }}
    }}
}}
"
    )
}

const DEFAULTS_AS_WRITTEN: &str =
    "the defaults are the #[prop] initialisers as written in the .wisp";

fn component_struct(name: &str, info: &ScriptInfo) -> String {
    let mut fields = String::new();
    for field in &info.fields {
        let _ = writeln!(
            fields,
            "    /// `{0}`, declared in `<script>`.\n    pub {0}: {1},",
            field.name, field.ty
        );
    }
    format!(
        "/// `{name}`: its signals, and what `unmount` removes. Dropping it
/// disposes its signals and effects, unless the component that mounted
/// it as a child owns it (`owned_by_parent`).
#[must_use = \"dropping a top-level component disposes its effects; keep it or call unmount\"]
pub struct {name} {{
{fields}    owner: Owner,
    roots: Vec<Root>,
    /// False once its parent's generated code took it as its child
    /// (`owned_by_parent`): its owner then goes with the parent's.
    disposes_on_drop: bool,
}}
"
    )
}

/// `(a, b, roots)` returned from the owner's closure, and the struct
/// literal fields that take them.
fn field_lists(info: &ScriptInfo) -> (String, String) {
    let fields: Vec<&str> = info
        .fields
        .iter()
        .map(|field| field.name.as_str())
        .collect();
    if fields.is_empty() {
        return ("roots".to_string(), "owner, roots".to_string());
    }
    let names = fields.join(", ");
    (
        format!("({names}, roots)"),
        format!("{names}, owner, roots"),
    )
}

/// The mount methods; `add_style` is the line that registers the
/// component's sheet, or empty.
fn mount_methods(name: &str, info: &ScriptInfo, body: &str, add_style: &str) -> String {
    let (returned, built) = field_lists(info);
    let allowances = mount_allowances(info, body);
    let prop_names: Vec<&str> = info.props.iter().map(|prop| prop.name.as_str()).collect();
    let prop_names = prop_names.join(", ");
    let mut statements = String::new();
    for statement in &info.statements {
        let _ = writeln!(statements, "            {statement}");
    }
    // a style-only component builds nothing: no place, no roots pushed
    let (unplaced, roots) = if body.trim().is_empty() {
        (
            "        let _ = (parent, before);\n",
            "#[allow(clippy::let_and_return, reason = \"a style-only component has no roots\")] let roots = Vec::new();",
        )
    } else {
        ("", "let mut roots = Vec::new();")
    };
    format!(
        "impl {name} {{
    /// Mounts the component at the end of `parent`.
    pub fn mount<D: Dom + ?Sized + 'static>(dom: &Rc<RefCell<D>>, parent: NodeId, props: {name}Props) -> Self
    where
        Rc<RefCell<D>>: DynDom,
    {{
        Self::mount_before(dom, parent, None, props)
    }}

    /// Mounts the component in `parent` before `before` (at the end when `None`).
{allowances}    pub fn mount_before<D: Dom + ?Sized + 'static>(dom: &Rc<RefCell<D>>, parent: NodeId, before: Option<NodeId>, props: {name}Props) -> Self
    where
        Rc<RefCell<D>>: DynDom,
    {{
        let {name}Props {{ {prop_names} }} = props;
{unplaced}{add_style}        dom.borrow_mut().declare_events(&Self::events());
        let disposes_on_drop = true;
        let owner = Owner::new();
        let {returned} = owner.with(|| {{
{statements}            {roots}
{body}            {returned}
        }});
        {name} {{ {built}, disposes_on_drop }}
    }}
"
    )
}
