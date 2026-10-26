//! A component tag checked against the props its component declares
//! (`PropsIndex`): each attribute, the tag's children (the `children`
//! prop) and each snippet declared in the tag must be a prop of the
//! component, or the tag is refused naming the prop and the file that
//! would declare it.

use super::emit::Emitter;
use super::props_index::Declared;
use crate::CompileError;
use crate::ast::{ComponentTag, Node};

impl Emitter<'_> {
    pub(super) fn check_props(&self, tag: &ComponentTag) -> Result<(), CompileError> {
        let Some(declared) = self.index.and_then(|index| index.of(&tag.name)) else {
            return Ok(());
        };
        let missing = passed(tag)
            .into_iter()
            .find(|name| !declared.props.iter().any(|prop| prop == name));
        match missing {
            Some(name) => Err(self
                .source
                .error(tag.offset, refusal(&tag.name, &name, declared))),
            None => Ok(()),
        }
    }
}

/// The props a tag passes: its attributes, `children` for any content,
/// and each snippet declared in it.
fn passed(tag: &ComponentTag) -> Vec<String> {
    let mut names: Vec<String> = tag
        .attributes
        .iter()
        .map(|attribute| {
            attribute
                .name
                .strip_prefix("bind:")
                .unwrap_or(&attribute.name)
                .to_owned()
        })
        .collect();
    for child in &tag.children {
        match child {
            Node::Snippet(snippet) => names.push(snippet.name.clone()),
            _ if !names.iter().any(|name| name == "children") => names.push("children".to_owned()),
            _ => {}
        }
    }
    names
}

fn refusal(tag: &str, name: &str, declared: &Declared) -> String {
    let props = if declared.props.is_empty() {
        "no props".to_owned()
    } else {
        format!("its props {}", declared.props.join(", "))
    };
    let declaration = match name {
        "children" => "`#[prop] let children: Snippet = Snippet::empty();`".to_owned(),
        _ => format!("`#[prop] let {name}: T = default;`"),
    };
    format!(
        "`<{tag}>` passes `{name}`, which {} does not declare ({props}); declare it there with {declaration}",
        declared.file
    )
}
