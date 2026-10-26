//! The clones a `move` closure needs of the script's plain bindings
//! and the snippets declared in the template.

use std::collections::HashSet;

use super::Emitter;
use crate::CompileError;
use crate::ast::Expr;
use crate::codegen::allowances::Allowance;
use crate::codegen::syn_map::parse_expr;
use crate::codegen::unwrap::identifiers;

impl Emitter<'_> {
    /// The script's plain bindings an expression mentions; a `move`
    /// closure that reads them gets a clone of each first.
    pub fn clones_for(&self, exprs: &[&Expr]) -> Result<Vec<String>, CompileError> {
        let mut mentioned = HashSet::new();
        for expr in exprs {
            mentioned.extend(identifiers(&parse_expr(self.source, expr)?));
        }
        let loop_names: HashSet<&String> = self.scopes.iter().flatten().collect();
        let mut clones: Vec<String> = self
            .script
            .bindings
            .iter()
            .chain(&self.snippets)
            .filter(|name| mentioned.contains(*name) && !loop_names.contains(name))
            .cloned()
            .collect();
        clones.sort();
        Ok(clones)
    }

    /// `let name = name.clone();` for each clone.
    pub fn emit_clones(&mut self, clones: &[String]) {
        for name in clones {
            self.line(Allowance::CloneOnCopy.attribute());
            self.line(format!("let {name} = {name}.clone();"));
        }
    }
}
