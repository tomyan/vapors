//! The props each component in a compiled directory declares (1-R1
//! review, finding 3), so a component tag can be checked against the
//! component it names: an attribute, children, or a snippet in the tag
//! that the component does not declare is a `.wisp` error, not a rustc
//! error in generated code. Components are keyed by their module path
//! under the directory, whose `wisp.rs` is included at the crate root,
//! and a tag is resolved to one by its path or the script's `use`
//! (`tag_path`); a tag that resolves to none is not checked, and the
//! build lists it.

use std::collections::HashMap;

use super::script;
use super::script_uses::UseName;
use super::tag_path::crate_path;
use crate::CompileError;
use crate::ast::Component;
use crate::source::Source;

/// Each component's declared props, by its path from the crate root.
#[derive(Debug, Default)]
pub struct PropsIndex {
    components: HashMap<Vec<String>, Declared>,
    /// Each file's module path, for the `self` and `super` in its tags.
    modules: HashMap<String, Vec<String>>,
}

/// What a component declares, and where.
#[derive(Debug, Clone)]
pub(crate) struct Declared {
    pub file: String,
    pub props: Vec<String>,
}

impl PropsIndex {
    /// Records the props of `component`, whose file is the module
    /// `module` (`["widgets", "card"]` for `widgets/card.wisp`).
    ///
    /// # Errors
    /// A script that does not parse, as [`crate::generate`] reports it;
    /// or a second component at one path, naming both files.
    pub fn add(&mut self, component: &Component, module: &[String]) -> Result<(), CompileError> {
        let source = Source::new(&component.path, &component.text);
        let info = script::analyse(&source, component.script.as_ref())?;
        let mut key = module.to_vec();
        key.push(component.name.clone());
        if let Some(first) = self.components.get(&key) {
            return Err(source.error(
                0,
                format!(
                    "`crate::{}` is also generated from {}; rename one of the files",
                    key.join("::"),
                    first.file
                ),
            ));
        }
        let props = info.props.iter().map(|prop| prop.name.clone()).collect();
        self.components.insert(
            key,
            Declared {
                file: component.path.clone(),
                props,
            },
        );
        self.modules.insert(component.path.clone(), module.to_vec());
        Ok(())
    }

    /// What the component a tag in `file` names declares, when the tag
    /// resolves to a component in the index.
    pub(crate) fn of(&self, file: &str, uses: &[UseName], tag: &str) -> Option<&Declared> {
        let module = self.modules.get(file)?;
        self.components.get(&crate_path(module, uses, tag)?)
    }
}
