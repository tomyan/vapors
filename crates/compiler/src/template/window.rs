//! `<vapors:window onkeydown={…} onkeyup={…} />`: page-level key
//! listeners. Only at the top level of a template, once, self-closing,
//! with only those two listeners; every other `vapors:` tag is refused.

use super::Parser;
use super::children::is_tag_char;
use crate::CompileError;
use crate::ast::{AttrValue, Attributes, WindowTag};

/// The listeners a window tag takes.
const LISTENERS: [&str; 2] = ["onkeydown", "onkeyup"];

const ONLY_WINDOW: &str =
    "the only `vapors:` tag is `<vapors:window onkeydown={handler} onkeyup={handler} />`";

impl Parser<'_> {
    /// Refuses the `vapors:` tag at the cursor (`<`) where it is not a
    /// top-level window tag: inside an element, block or snippet, or
    /// not `window` at all. Other names are left to the caller.
    pub fn refuse_vapors_tag(&self, name: &str, offset: usize) -> Result<(), CompileError> {
        match name {
            "vapors:window" => Err(self.error(offset, "`<vapors:window>` goes at the top level of the template, not inside an element, `{if}`, `{for}`, snippet or component tag: move it out to the template's top, beside the root elements")),
            _ if name.starts_with("vapors:") => Err(self.unknown_vapors_tag(name, offset)),
            _ => Ok(()),
        }
    }

    fn unknown_vapors_tag(&self, name: &str, offset: usize) -> CompileError {
        self.error(offset, format!("unknown tag `<{name}>`: {ONLY_WINDOW}"))
    }

    /// Parses the window tag at the cursor (`<`), top level.
    pub fn parse_window(&mut self, first: Option<usize>) -> Result<WindowTag, CompileError> {
        let offset = self.pos;
        self.bump(1);
        let name = self.read_while(is_tag_char).to_string();
        if name != "vapors:window" {
            return Err(self.unknown_vapors_tag(&name, offset));
        }
        if let Some(first) = first {
            let at = self.describe(first);
            return Err(self.error(offset, format!("`<vapors:window>` is already at {at}: a component has one; put both listeners on it")));
        }
        let attributes = self.parse_attributes(&name)?;
        self.expect_window_end(offset)?;
        self.check_window_attributes(&attributes, offset)?;
        Ok(WindowTag { attributes, offset })
    }

    fn expect_window_end(&mut self, offset: usize) -> Result<(), CompileError> {
        if self.starts_with("/>") {
            self.bump(2);
            return Ok(());
        }
        Err(self.error(offset, "`<vapors:window>` has no children: write it self-closing, `<vapors:window onkeydown={handler} />`"))
    }

    fn check_window_attributes(
        &self,
        attributes: &Attributes,
        offset: usize,
    ) -> Result<(), CompileError> {
        if attributes.is_empty() {
            return Err(self.error(offset, "`<vapors:window>` has no listener, so it does nothing: add `onkeydown={handler}` or `onkeyup={handler}`"));
        }
        for attribute in attributes.iter() {
            if !LISTENERS.contains(&attribute.name.as_str()) {
                return Err(self.error(attribute.offset, format!("`{}` is not supported on `<vapors:window>`: it takes `onkeydown={{handler}}` and `onkeyup={{handler}}` only", attribute.name)));
            }
            if matches!(attribute.value, AttrValue::Literal(_)) {
                return Err(self.error(
                    attribute.offset,
                    format!(
                        "`{0}` takes an expression: write `{0}={{handler}}`, not a string",
                        attribute.name
                    ),
                ));
            }
        }
        Ok(())
    }
}
