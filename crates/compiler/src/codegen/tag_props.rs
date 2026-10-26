//! A component tag checked against the props its component declares
//! (`PropsIndex`): each attribute, the tag's children (the `children`
//! prop) and each snippet declared in the tag must be a prop of the
//! component. Each one that is not is refused at its attribute, its
//! snippet or the tag, naming the prop, the file that would declare it,
//! and a declaration that compiles. A tag the index cannot resolve is
//! recorded as unchecked, for the build to list.

use super::emit::Emitter;
use super::props_index::{Declared, PropsIndex};
use crate::CompileError;
use crate::ast::{AttrValue, Attribute, ComponentTag, Node};

/// The props check's state while a component is emitted.
#[derive(Default)]
pub(crate) struct TagCheck<'a> {
    /// `None` when nothing is checked (a single file's `generate`).
    pub index: Option<&'a PropsIndex>,
    pub errors: Vec<CompileError>,
    /// `file:line:col: ..` for each tag not checked.
    pub unchecked: Vec<String>,
}

/// The prop an attribute sets: its name without `bind:`, which passes
/// the signal itself. The check and the generated props struct both
/// read it here.
pub(super) fn prop_name(attribute: &Attribute) -> &str {
    attribute
        .name
        .strip_prefix("bind:")
        .unwrap_or(&attribute.name)
}

/// A prop a tag passes, where, and the declaration it needs.
struct Passed {
    name: String,
    offset: usize,
    declaration: String,
    /// Whether the declaration's type is `()`, for the author to replace.
    placeholder: bool,
}

impl Emitter<'_> {
    pub(super) fn check_props(&mut self, tag: &ComponentTag) {
        let Some(index) = self.tags.index else {
            return;
        };
        let Some(declared) = index.of(&self.source.path, &self.script.uses, &tag.name) else {
            self.note_unchecked(tag);
            return;
        };
        for passed in passed(tag) {
            if !declared.props.contains(&passed.name) {
                let refusal = refusal(&tag.name, &passed, declared);
                self.tags
                    .errors
                    .push(self.source.error(passed.offset, refusal));
            }
        }
    }
}

impl Emitter<'_> {
    fn note_unchecked(&mut self, tag: &ComponentTag) {
        let at = self.source.describe(tag.offset);
        self.tags.unchecked.push(format!(
            "{}:{at}: `<{}>` is not a component in this directory, so its props are not checked",
            self.source.path, tag.name
        ));
    }
}

/// The props a tag passes: its attributes, `children` for any content,
/// and each snippet declared in it.
fn passed(tag: &ComponentTag) -> Vec<Passed> {
    let mut passed: Vec<Passed> = tag.attributes.0.iter().map(attribute_passed).collect();
    for child in &tag.children {
        match child {
            Node::Snippet(snippet) => passed.push(Passed {
                name: snippet.name.clone(),
                offset: snippet.offset,
                declaration: super::prop_suggestion::snippet(&snippet.name, &snippet.params),
                placeholder: false,
            }),
            _ if !passed.iter().any(|p| p.name == "children") => passed.push(Passed {
                name: "children".to_owned(),
                offset: tag.offset,
                declaration: "#[prop] let children: Snippet = Snippet::empty();".to_owned(),
                placeholder: false,
            }),
            _ => {}
        }
    }
    passed
}

fn attribute_passed(attribute: &Attribute) -> Passed {
    let name = prop_name(attribute).to_owned();
    let (declaration, placeholder) = match (&attribute.value, attribute.name.starts_with("bind:")) {
        (_, true) => super::prop_suggestion::bound(&name),
        (AttrValue::Literal(_), false) => (
            format!("#[prop] let {name}: String = String::new();"),
            false,
        ),
        (AttrValue::Expr(expr), false) => super::prop_suggestion::value(&name, &expr.code),
    };
    Passed {
        name,
        offset: attribute.offset,
        declaration,
        placeholder,
    }
}

fn refusal(tag: &str, passed: &Passed, declared: &Declared) -> String {
    let props = if declared.props.is_empty() {
        "no props".to_owned()
    } else {
        format!("its props {}", declared.props.join(", "))
    };
    format!(
        "`<{tag}>` passes `{}`, which {} does not declare ({props}); declare it there with `{}`{}",
        passed.name,
        declared.file,
        passed.declaration,
        if passed.placeholder {
            " and replace `()` with the prop's type"
        } else {
            ""
        }
    )
}
