//! The file around the template code: imports, the style constant,
//! the props and component structs, and the mount and unmount methods.

use std::fmt::Write;

use super::allowances::Allowance;
use super::script::ScriptInfo;
use crate::ast::Component;

/// The whole generated file, before prettyplease lays it out.
pub(crate) fn assemble(component: &Component, info: &ScriptInfo, body: &str) -> String {
    let name = &component.name;
    let style = component
        .style
        .as_ref()
        .map_or("", |section| section.text.as_str());
    let mut file = header(&component.path, style);
    let scope = component.style.as_ref().and(component.scope.as_ref());
    if let Some(scope) = scope {
        file.push_str(&scope_constant(scope));
    }
    file.push_str(&props_struct(name, info));
    file.push_str(&component_struct(name, info));
    file.push_str(&mount_methods(name, info, body, scope.is_some()));
    file.push_str(&accessors());
    file
}

fn header(path: &str, style: &str) -> String {
    format!(
        "use std::cell::RefCell;
use std::rc::Rc;
#[allow(unused_imports)]
use vapors_dom::{{Dom, Event, NodeId, PropertyValue, Root, SCOPE_ATTRIBUTE}};
#[allow(unused_imports)]
use vapors_signals::{{Effect, Memo, Owner, Signal}};

/// The `<style>` of `{path}`, resolved at run time by the css crate.
pub const STYLE: &str = {style:?};

"
    )
}

/// The scope the style is added under and each element is tagged with:
/// the crate and the file, so two crates' `src/panel.wisp` differ.
fn scope_constant(scope: &str) -> String {
    format!(
        "/// The scope [`STYLE`] is added under, set on each element as [`SCOPE_ATTRIBUTE`].
pub const SCOPE: &str = concat!(env!(\"CARGO_PKG_NAME\"), \"/\", {scope:?});

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
            "    /// `{0}`, declared with `#[prop]`.\n    pub {0}: {1},",
            prop.name, prop.ty
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
/// Lints on `mount_before` that depend on types the compiler cannot see.
fn mount_allowances(info: &ScriptInfo) -> String {
    let mut allowances = format!("    {}\n", Allowance::TooManyLines.attribute());
    if info.props.is_empty() {
        let _ = writeln!(
            allowances,
            "    {}",
            Allowance::NeedlessPassByValue.attribute()
        );
    }
    allowances
}

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
        "/// `{name}`: its signals, and what `unmount` removes.
pub struct {name} {{
{fields}    owner: Owner,
    roots: Vec<Root>,
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

fn mount_methods(name: &str, info: &ScriptInfo, body: &str, styled: bool) -> String {
    let (returned, built) = field_lists(info);
    let allowances = mount_allowances(info);
    let prop_names: Vec<&str> = info.props.iter().map(|prop| prop.name.as_str()).collect();
    let prop_names = prop_names.join(", ");
    let add_style = if styled {
        "        dom.borrow_mut().add_style(SCOPE, STYLE);\n"
    } else {
        ""
    };
    let mut statements = String::new();
    for statement in &info.statements {
        let _ = writeln!(statements, "            {statement}");
    }
    format!(
        "impl {name} {{
    /// Mounts the component at the end of `parent`.
    pub fn mount<D: Dom + 'static>(dom: &Rc<RefCell<D>>, parent: NodeId, props: {name}Props) -> Self {{
        Self::mount_before(dom, parent, None, props)
    }}

    /// Mounts the component in `parent` before `before` (at the end when `None`).
{allowances}    pub fn mount_before<D: Dom + 'static>(dom: &Rc<RefCell<D>>, parent: NodeId, before: Option<NodeId>, props: {name}Props) -> Self {{
        let {name}Props {{ {prop_names} }} = props;
{add_style}        let owner = Owner::new();
        let {returned} = owner.with(|| {{
{statements}            let mut roots = Vec::new();
{body}            {returned}
        }});
        {name} {{ {built} }}
    }}
"
    )
}

fn accessors() -> String {
    "
    /// The top-level nodes now, in order, including those of a block
    /// or child component at the top level.
    #[must_use]
    pub fn roots(&self) -> Vec<NodeId> {
        vapors_dom::current_nodes(&self.roots)
    }

    /// The top-level entries, for a parent that mounts this component
    /// at its own top level.
    #[must_use]
    pub fn root_entries(&self) -> &[Root] {
        &self.roots
    }

    /// Reads the top-level nodes, disposes the signals and effects, and
    /// removes the nodes.
    pub fn unmount<D: Dom>(self, dom: &Rc<RefCell<D>>) {
        let nodes = self.roots();
        self.owner.dispose();
        for node in nodes {
            dom.borrow_mut().remove(node);
        }
    }
}
"
    .to_string()
}
