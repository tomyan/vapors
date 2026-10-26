//! The props each component in a compiled directory declares (1-R1
//! review, finding 3), so a component tag can be checked against the
//! component it names: an attribute, children, or a snippet in the tag
//! that the component does not declare is a `.wisp` error at the tag,
//! not a rustc error in generated code. A component outside the
//! directory, or a name two of its components share, is not checked;
//! rustc reports those as before.

use std::collections::HashMap;

use super::script;
use crate::CompileError;
use crate::ast::Component;
use crate::source::Source;

/// Each component's declared props, by its type name.
#[derive(Debug, Default)]
pub struct PropsIndex {
    /// `None` for a name two components share.
    components: HashMap<String, Option<Declared>>,
}

/// What a component declares, and where.
#[derive(Debug, Clone)]
pub(crate) struct Declared {
    pub file: String,
    pub props: Vec<String>,
}

impl PropsIndex {
    /// Records `component`'s props.
    ///
    /// # Errors
    /// A script that does not parse, as [`crate::generate`] reports it.
    pub fn add(&mut self, component: &Component) -> Result<(), CompileError> {
        let source = Source::new(&component.path, &component.text);
        let info = script::analyse(&source, component.script.as_ref())?;
        let declared = Declared {
            file: component.path.clone(),
            props: info.props.iter().map(|prop| prop.name.clone()).collect(),
        };
        let shared = self.components.contains_key(&component.name);
        self.components
            .insert(component.name.clone(), (!shared).then_some(declared));
        Ok(())
    }

    /// What the component a tag names declares, when it is in the index
    /// once: `Card` or `widgets::Card` both look up `Card`.
    pub(crate) fn of(&self, tag: &str) -> Option<&Declared> {
        let name = tag.rsplit("::").next().unwrap_or(tag);
        self.components.get(name).and_then(Option::as_ref)
    }
}
