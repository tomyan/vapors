//! Text runs: a literal is one text node created once; a `{expr}` hole
//! is a text node of its own with an effect that re-sets exactly it.

use super::allowances::Allowance;
use super::emit::{Emitter, Place};
use crate::CompileError;
use crate::ast::{Expr, Part, Text};

impl Emitter<'_> {
    pub fn emit_text(&mut self, text: &Text, place: &Place) -> Result<(), CompileError> {
        for part in &text.parts {
            match part {
                Part::Literal(value) => {
                    let var = self.fresh("text");
                    self.line(format!(
                        "let {var} = dom.borrow_mut().create_text({value:?});"
                    ));
                    self.attach(&var, place);
                }
                Part::Expr(expr) => self.emit_hole(expr, place)?,
            }
        }
        Ok(())
    }

    fn emit_hole(&mut self, expr: &Expr, place: &Place) -> Result<(), CompileError> {
        let stem = match identifier(&expr.code) {
            Some(name) => format!("{name}_text"),
            None => "text".to_string(),
        };
        let var = self.fresh(&stem);
        self.line(format!("let {var} = dom.borrow_mut().create_text(\"\");"));
        self.attach(&var, place);
        let value = self.unwrapped(expr)?;
        let clones = self.clones_for(&[expr])?;
        self.allow(Allowance::ImplicitClone);
        self.emit_effect(
            &clones,
            &[
                format!("let value = ({value}).to_string();"),
                format!("dom.borrow_mut().set_text({var}, &value);"),
            ],
        );
        Ok(())
    }

    /// `{ let dom = Rc::clone(dom); clones; Effect::new(move || { body }); }`
    pub fn emit_effect(&mut self, clones: &[String], body: &[String]) {
        self.line("{");
        self.indent(1);
        self.line("let dom = Rc::clone(dom);");
        self.emit_clones(clones);
        self.line("Effect::new(move || {");
        self.indent(1);
        for line in body {
            self.line(line);
        }
        self.dedent(1);
        self.line("});");
        self.dedent(1);
        self.line("}");
    }
}

/// The name when the code is a bare identifier.
fn identifier(code: &str) -> Option<&str> {
    let mut chars = code.chars();
    let first = chars.next()?;
    let is_identifier =
        (first.is_alphabetic() || first == '_') && chars.all(|c| c.is_alphanumeric() || c == '_');
    is_identifier.then_some(code)
}
