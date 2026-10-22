//! The file around the template code: imports, the style constant,
//! the props and component structs, and the mount and unmount methods.

use std::fmt::Write;

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
    file.push_str(&props_struct(name, info));
    file.push_str(&component_struct(name, info));
    file.push_str(&mount_methods(name, info, body));
    file.push_str(&accessors());
    file
}

fn header(path: &str, style: &str) -> String {
    format!(
        "use std::cell::RefCell;
use std::rc::Rc;
#[allow(unused_imports)]
use vapors_dom::{{Dom, Event, NodeId, PropertyValue}};
#[allow(unused_imports)]
use vapors_signals::{{Effect, Memo, Owner, Signal}};

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
const CLONES: &str =
    "a closure gets a clone of each binding it captures; the compiler cannot see which are Copy";
const PROPS_BY_VALUE: &str =
    "every component takes its props struct by value, empty or not, so parents mount them alike";

/// Lints on `mount_before` that depend on types the compiler cannot see.
fn mount_allowances(info: &ScriptInfo) -> String {
    let mut allowances = format!("    #[allow(clippy::clone_on_copy, reason = \"{CLONES}\")]\n");
    if info.props.is_empty() {
        let _ = writeln!(
            allowances,
            "    #[allow(clippy::needless_pass_by_value, reason = \"{PROPS_BY_VALUE}\")]"
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
    roots: Vec<NodeId>,
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

fn mount_methods(name: &str, info: &ScriptInfo, body: &str) -> String {
    let (returned, built) = field_lists(info);
    let allowances = mount_allowances(info);
    let prop_names: Vec<&str> = info.props.iter().map(|prop| prop.name.as_str()).collect();
    let prop_names = prop_names.join(", ");
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
        let owner = Owner::new();
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
    /// The top-level nodes mounted, in order.
    #[must_use]
    pub fn roots(&self) -> &[NodeId] {
        &self.roots
    }

    /// Disposes the signals and effects and removes the nodes.
    pub fn unmount<D: Dom>(self, dom: &Rc<RefCell<D>>) {
        self.owner.dispose();
        for node in self.roots {
            dom.borrow_mut().remove(node);
        }
    }
}
"
    .to_string()
}
