//! `bind:this={signal}`: the element's node into the signal once the
//! element is built, children included, and out of it when the owner
//! it was built under (the component, an `{if}` branch, a `{for}` row)
//! is disposed, unless the signal holds another node by then, or was
//! disposed with it. Outside `{for}` the signal is
//! `Signal<Option<NodeId>>`; in a row, `Signal<BTreeMap<K, NodeId>>`
//! under the row's `key=`, which a reused row keeps.

use std::fmt::Write as _;

use super::emit::Emitter;
use super::syn_map::parse_expr;
use super::unwrap::identifiers;
use crate::CompileError;
use crate::ast::{AttrValue, Attribute, Expr};

/// A `{for}` row being built: its loop names and its `key=`.
#[derive(Clone)]
pub(crate) struct RowKey {
    pub index: String,
    pub item: String,
    pub key: Option<Expr>,
}

const SINGLE: &str = "Signal<Option<NodeId>>";
const KEYED: &str = "Signal<BTreeMap<K, NodeId>>";

impl Emitter<'_> {
    pub(super) fn emit_bind_this(
        &mut self,
        var: &str,
        attribute: &Attribute,
    ) -> Result<(), CompileError> {
        let AttrValue::Expr(expr) = &attribute.value else {
            return Err(self.source.error(
                attribute.offset,
                format!("`bind:this` takes a signal of `Option<NodeId>`: write bind:this={{signal}}, a `{SINGLE}`"),
            ));
        };
        let row = self.bound_row(attribute)?;
        self.check_bind_this_type(expr, attribute, row.is_some())?;
        let signal = self.raw(expr)?;
        match row {
            None => self.emit_bind_this_single(var, &signal),
            Some(row) => self.emit_bind_this_keyed(var, &signal, &row, attribute)?,
        }
        Ok(())
    }

    /// The `{for}` row the element is in, if any; a nested `{for}` is
    /// refused.
    fn bound_row(&self, attribute: &Attribute) -> Result<Option<RowKey>, CompileError> {
        match self.row_keys.as_slice() {
            [] => Ok(None),
            [row] => Ok(Some(row.clone())),
            _ => Err(self.source.error(attribute.offset, "`bind:this` in a nested `{for}`: a map keyed by the inner row's key would mix the outer rows' nodes; bind in the outer `{for}`, or move the inner one into a component")),
        }
    }

    fn emit_bind_this_single(&mut self, var: &str, signal: &str) {
        // a signal disposed with the same owner (the component's own) is
        // gone, and there is nothing to clear: `try_` reads skip it
        self.line(format!(
            "{{ let this = {signal}; this.set(Some({var})); vapors_signals::on_cleanup(move || {{ if this.try_get_untracked() == Ok(Some({var})) {{ this.set(None); }} }}); }}"
        ));
    }

    fn emit_bind_this_keyed(
        &mut self,
        var: &str,
        signal: &str,
        row: &RowKey,
        attribute: &Attribute,
    ) -> Result<(), CompileError> {
        let Some(key) = &row.key else {
            return Err(self.source.error(attribute.offset, format!(
                "`bind:this` in a `{{for}}` without `key=`: the row's node is kept under its key in a `{KEYED}`; add key=... to the `{{for}}`"
            )));
        };
        let key = self.row_key_expr(key, row)?;
        self.line(format!(
            "{{ let this = {signal}; let key = {key}; this.update(|nodes| {{ nodes.insert(key.clone(), {var}); }}); vapors_signals::on_cleanup(move || {{ if this.try_with_untracked(|nodes| nodes.get(&key) == Some(&{var})) == Ok(true) {{ this.update(|nodes| {{ nodes.remove(&key); }}); }} }}); }}"
        ));
        Ok(())
    }

    /// The row's key, from its item and index as built: a reused row's
    /// key is the same by definition, so reading them once is enough.
    fn row_key_expr(&self, key: &Expr, row: &RowKey) -> Result<String, CompileError> {
        let code = self.raw(key)?;
        let mentioned = identifiers(&parse_expr(self.source, key)?);
        let names: Vec<&String> = [&row.index, &row.item]
            .into_iter()
            .filter(|name| mentioned.contains(*name))
            .collect();
        if let [name] = names.as_slice()
            && code.trim() == name.as_str()
        {
            return Ok(format!("{name}.get_untracked()"));
        }
        let reads = names.iter().fold(String::new(), |mut reads, name| {
            let _ = write!(reads, "let {name} = {name}.get_untracked(); ");
            reads
        });
        Ok(format!("{{ {reads}{code} }}"))
    }

    /// A script signal or prop of the wrong shape for where it is bound
    /// is refused here, naming the shape; other expressions are left to
    /// rustc.
    fn check_bind_this_type(
        &self,
        expr: &Expr,
        attribute: &Attribute,
        in_row: bool,
    ) -> Result<(), CompileError> {
        let Some(ty) = self.declared_type(&self.raw(expr)?) else {
            return Ok(());
        };
        let (wanted, fits, why) = if in_row {
            (
                KEYED,
                is_keyed(&ty),
                "in a `{for}` row one `Option<NodeId>` cannot hold a node per row; a map holds each row's node under its key=",
            )
        } else {
            (
                SINGLE,
                is_single(&ty),
                "outside a `{for}` there is one node",
            )
        };
        if fits {
            return Ok(());
        }
        Err(self.source.error(
            attribute.offset,
            format!("`bind:this` takes a `{wanted}` here, not `{ty}`: {why}"),
        ))
    }

    /// The type a script signal or prop named `name` is declared with,
    /// without whitespace.
    fn declared_type(&self, name: &str) -> Option<String> {
        let script = self.script;
        let fields = script.fields.iter().map(|field| (&field.name, &field.ty));
        let props = script.props.iter().map(|prop| (&prop.name, &prop.ty));
        fields
            .chain(props)
            .find(|(declared, _)| declared.as_str() == name.trim())
            .map(|(_, ty)| ty.chars().filter(|c| !c.is_whitespace()).collect())
    }
}

fn is_single(ty: &str) -> bool {
    ty.starts_with("Signal<Option<") && ty.ends_with("NodeId>>")
}

fn is_keyed(ty: &str) -> bool {
    ty.starts_with("Signal<") && ty.contains("BTreeMap<") && ty.ends_with("NodeId>>")
}
