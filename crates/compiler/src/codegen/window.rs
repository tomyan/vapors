//! `<vapors:window>`: each listener is added to the page when the
//! component mounts and removed when its owner is disposed (unmount).

use super::emit::Emitter;
use super::emit::Place;
use crate::CompileError;
use crate::ast::{AttrValue, Attribute, Document};

impl Emitter<'_> {
    /// The template's nodes, then its window listeners.
    pub fn emit_template_nodes(&mut self, template: &Document) -> Result<(), CompileError> {
        self.emit_nodes(&template.children, &Place::roots())?;
        let Some(window) = &template.window else {
            return Ok(());
        };
        window
            .attributes
            .iter()
            .try_for_each(|attribute| self.emit_window_listener(attribute))
    }

    fn emit_window_listener(&mut self, attribute: &Attribute) -> Result<(), CompileError> {
        let AttrValue::Expr(expr) = &attribute.value else {
            unreachable!("the parser refuses a window listener that is not an expression");
        };
        let event = attribute.name.trim_start_matches("on");
        let handler = self.handler_expr(expr)?;
        let clones = self.clones_for(&[expr])?;
        self.line("{");
        self.indent(1);
        self.emit_clones(&clones);
        self.line(format!(
            "let listener = dom.borrow_mut().listen_window({event:?}, Box::new({handler}));"
        ));
        self.line("let dom = Rc::clone(dom);");
        self.line(
            "vapors_signals::on_cleanup(move || { dom.borrow_mut().unlisten_window(listener); });",
        );
        self.dedent(1);
        self.line("}");
        Ok(())
    }
}
