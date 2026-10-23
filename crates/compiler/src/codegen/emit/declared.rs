//! The names the emitter generates and the names the template declares
//! in loops, so a collision between them is refused (see `names`).

use std::collections::HashSet;

use super::Emitter;
use crate::CompileError;
use crate::codegen::names::check_declared;

impl Emitter<'_> {
    /// Records a name the generated code binds, so a script or loop
    /// binding of the same name is refused.
    pub fn reserve(&mut self, name: &str) {
        self.generated.insert(name.to_string());
    }

    pub fn generated(&self) -> &HashSet<String> {
        &self.generated
    }

    /// The script's names and the loop bindings, with their positions.
    pub fn declared(&self) -> Vec<(String, usize)> {
        let mut declared = self.script.declared.clone();
        declared.extend(self.loop_bindings.iter().cloned());
        declared
    }

    /// Records a loop binding, refusing a reserved or raw name.
    pub fn declare_loop_binding(&mut self, name: &str, offset: usize) -> Result<(), CompileError> {
        check_declared(self.source, name, offset)?;
        self.loop_bindings.push((name.to_string(), offset));
        Ok(())
    }
}
