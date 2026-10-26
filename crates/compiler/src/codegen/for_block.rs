//! `{for pattern in iterable key=expr}`: an anchor, the items closure
//! that lists `(key, item)` pairs, and a row builder, handed to
//! `vapors_dom::blocks::keyed` with the block's position for refusals.

use super::emit::{Emitter, Place};
use crate::CompileError;
use crate::ast::ForBlock;

impl Emitter<'_> {
    pub fn emit_for(&mut self, block: &ForBlock, place: &Place) -> Result<(), CompileError> {
        let anchor = self.fresh("for_anchor");
        self.emit_anchor(&anchor, place);
        let (index, item) = self.loop_names(block)?;
        for name in [&index, &item] {
            if name != "_index" {
                self.declare_loop_binding(name, block.offset)?;
            }
        }
        let row_scope = vec![index.trim_start_matches('_').to_string(), item.clone()];
        let row_clones = self.row_clones(block, row_scope.clone())?;
        let call = self.open_keyed_call(place, &row_clones);
        self.emit_keyed_items(block, &anchor, place, &index, &item)?;
        let signature =
            self.builder_signature(&format!(", {index}: Signal<usize>, {item}: Signal<_>"));
        self.push_scope(row_scope);
        let built = self.emit_builder_with(&[], &block.children, &signature, &[]);
        self.pop_scope();
        built?;
        self.close_keyed_call(call);
        Ok(())
    }

    /// What the row reads with the loop's names in scope, which shadow
    /// the component's.
    fn row_clones(
        &mut self,
        block: &ForBlock,
        scope: Vec<String>,
    ) -> Result<Vec<String>, CompileError> {
        self.push_scope(scope);
        let clones = self.builder_clones(&block.children);
        self.pop_scope();
        clones
    }

    /// The arguments before the row: the Dom, the place, the site named
    /// in refusals, and the items closure.
    fn emit_keyed_items(
        &mut self,
        block: &ForBlock,
        anchor: &str,
        place: &Place,
        index: &str,
        item: &str,
    ) -> Result<(), CompileError> {
        let iterable = self.raw(&block.iterable)?;
        let key = block.key.as_ref().map(|key| self.raw(key)).transpose()?;
        let site = format!(
            "{}:{}",
            self.source.path,
            self.source.describe(block.offset)
        );
        self.line(format!("dom, {}, {anchor}, {site:?},", place.parent));
        let mut read_by_items = vec![&block.iterable];
        read_by_items.extend(block.key.as_ref());
        let clones = self.clones_for(&read_by_items)?;
        self.emit_closure(
            &clones,
            &[keyed_items(&iterable, index, item, key.as_deref())],
        );
        Ok(())
    }

    /// Opens the `keyed` call, with the row's clones taken before it in
    /// a block around the call. The row closure must stay an argument
    /// of `keyed` itself: rustc checks a block argument before the
    /// closure arguments, and a row inside one would be checked before
    /// the items closure tells it the item type (E0282 on `V`).
    fn open_keyed_call(&mut self, place: &Place, row_clones: &[String]) -> KeyedCall {
        if row_clones.is_empty() {
            return KeyedCall::Plain(self.open_block_call("keyed", place));
        }
        let handle = place.roots.then(|| self.fresh("keyed_block"));
        self.line(match &handle {
            Some(handle) => format!("let {handle} = {{"),
            None => "{".to_string(),
        });
        self.indent(1);
        self.emit_clones(row_clones);
        self.line("vapors_dom::blocks::keyed(");
        self.indent(1);
        KeyedCall::Hoisted(handle)
    }

    fn close_keyed_call(&mut self, call: KeyedCall) {
        let handle = match call {
            KeyedCall::Plain(handle) => return self.close_block_call(handle),
            KeyedCall::Hoisted(handle) => handle,
        };
        self.dedent(1);
        self.line(if handle.is_some() { ")" } else { ");" });
        self.dedent(1);
        self.line(if handle.is_some() { "};" } else { "}" });
        if let Some(handle) = handle {
            self.line(format!("roots.push(Root::Block({handle}));"));
        }
    }

    /// `(index, item)` enumerates; `item` alone gets `_index`.
    fn loop_names(&self, block: &ForBlock) -> Result<(String, String), CompileError> {
        let pattern = block.pattern.trim();
        let inner = pattern
            .strip_prefix('(')
            .and_then(|rest| rest.strip_suffix(')'));
        let names: Vec<&str> = match inner {
            Some(inner) => inner.split(',').map(str::trim).collect(),
            None => vec!["_index", pattern],
        };
        if names.len() == 2 && names.iter().all(|name| is_identifier(name)) {
            return Ok((names[0].to_string(), names[1].to_string()));
        }
        Err(self.source.error(
            block.offset,
            format!("`{{for {pattern} in ...}}`: the pattern is `item` or `(index, item)`, two plain names"),
        ))
    }
}

/// A `keyed` call as opened, so it closes the same way; each holds the
/// handle kept among the roots, when kept.
enum KeyedCall {
    Plain(Option<String>),
    Hoisted(Option<String>),
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && chars.all(|c| c.is_alphanumeric() || c == '_')
}

/// The closure body listing `(key, item)` pairs. Without a key the
/// index is the key; `enumerate` appears only when the index is read.
fn keyed_items(iterable: &str, index: &str, item: &str, key: Option<&str>) -> String {
    match key {
        Some(key) if index.starts_with('_') => {
            format!("move || ({iterable}).into_iter().map(|{item}| (({key}), {item})).collect(),")
        }
        Some(key) => format!(
            "move || ({iterable}).into_iter().enumerate().map(|({index}, {item})| (({key}), {item})).collect(),"
        ),
        None => format!(
            "move || ({iterable}).into_iter().enumerate().map(|({index}, {item})| ({index}, {item})).collect(),"
        ),
    }
}
