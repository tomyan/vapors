//! Attributes: `name="literal"`, `name={expr}` and the `{name}`
//! shorthand, each name once.

use super::Parser;
use crate::CompileError;
use crate::ast::{AttrValue, Attribute, Attributes, Expr};

impl Parser<'_> {
    /// Reads attributes up to the `>` or `/>` of the tag, leaving the
    /// cursor on it.
    pub fn parse_attributes(&mut self, tag: &str) -> Result<Attributes, CompileError> {
        let mut attributes = Attributes::default();
        loop {
            self.skip_whitespace();
            match self.peek() {
                None => {
                    return Err(self.error(
                        self.pos,
                        format!(
                            "the file ends inside the `<{tag}>` tag: finish it with `>` or `/>`"
                        ),
                    ));
                }
                Some('>' | '/') => return Ok(attributes),
                Some('{') => {
                    let attribute = self.parse_shorthand_attribute()?;
                    self.push_attribute(&mut attributes, attribute)?;
                }
                Some(_) => {
                    let attribute = self.parse_named_attribute(tag)?;
                    self.push_attribute(&mut attributes, attribute)?;
                }
            }
        }
    }

    fn push_attribute(
        &self,
        attributes: &mut Attributes,
        attribute: Attribute,
    ) -> Result<(), CompileError> {
        if attributes.get(&attribute.name).is_some() {
            return Err(self.error(
                attribute.offset,
                format!("attribute `{}` is given twice: keep one", attribute.name),
            ));
        }
        attributes.push(attribute);
        Ok(())
    }

    /// `{name}` is `name={name}`.
    fn parse_shorthand_attribute(&mut self) -> Result<Attribute, CompileError> {
        let hole = self.peek_hole()?;
        let is_identifier = hole.body.chars().all(|c| c.is_alphanumeric() || c == '_');
        if !is_identifier {
            return Err(self.error(
                hole.offset,
                format!(
                    "`{{{}}}` as an attribute is shorthand for `name={{name}}` and needs a plain name",
                    hole.body
                ),
            ));
        }
        self.bump(hole.len);
        Ok(Attribute {
            name: hole.body.clone(),
            value: AttrValue::Expr(Expr {
                code: hole.body,
                offset: hole.body_offset,
            }),
            offset: hole.offset,
        })
    }

    fn parse_named_attribute(&mut self, tag: &str) -> Result<Attribute, CompileError> {
        let offset = self.pos;
        let name = self
            .read_while(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | ':' | '.'))
            .to_string();
        if name.is_empty() {
            let found = self.peek().unwrap_or(' ');
            return Err(self.error(
                offset,
                format!("unexpected `{found}` in the `<{tag}>` tag: expected an attribute name"),
            ));
        }
        if !self.starts_with("=") {
            return Err(self.error(
                offset,
                format!(
                    "attribute `{name}` needs a value: write {name}=\"...\" or {name}={{expr}}"
                ),
            ));
        }
        self.bump(1);
        let value = self.parse_attribute_value(&name)?;
        Ok(Attribute {
            name,
            value,
            offset,
        })
    }

    fn parse_attribute_value(&mut self, name: &str) -> Result<AttrValue, CompileError> {
        let offset = self.pos;
        match self.peek() {
            Some('{') => {
                let hole = self.peek_hole().map_err(|err| CompileError {
                    message: format!("in the value of attribute `{name}`: {}", err.message),
                    ..err
                })?;
                self.bump(hole.len);
                Ok(AttrValue::Expr(Expr {
                    code: hole.body,
                    offset: hole.body_offset,
                }))
            }
            Some('"') => {
                self.bump(1);
                let value = self.read_while(|c| c != '"').to_string();
                if !self.starts_with("\"") {
                    return Err(self.error(
                        offset,
                        format!("the value of attribute `{name}` is missing its closing `\"`"),
                    ));
                }
                self.bump(1);
                Ok(AttrValue::Literal(value))
            }
            _ => Err(self.error(
                offset,
                format!("the value of attribute `{name}` must be \"quoted\" or {{an expression}}"),
            )),
        }
    }
}
