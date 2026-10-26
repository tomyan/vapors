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
        let iterable = self.raw(&block.iterable)?;
        let key = block.key.as_ref().map(|key| self.raw(key)).transpose()?;
        let handle = self.open_block_call("keyed", place);
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
            &[keyed_items(&iterable, &index, &item, key.as_deref())],
        );
        let signature =
            self.builder_signature(&format!(", {index}: Signal<usize>, {item}: Signal<_>"));
        self.push_scope(vec![index.trim_start_matches('_').to_string(), item]);
        let built = self.emit_builder(&block.children, &signature);
        self.pop_scope();
        built?;
        self.close_block_call(handle);
        Ok(())
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
