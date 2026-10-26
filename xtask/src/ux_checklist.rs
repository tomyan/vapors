//! The UX checklist (`ux.checklist`): Markdown whose list items are the
//! review's items, each `- measured <id>: <text>` or
//! `- judged <id>: <text>`. A measured item's id names a fact in the
//! walk's facts.json, filled in by the script; only judged items go to
//! the model. Any other list item is refused with its line.

/// Who answers an item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The script, from facts.json.
    Measured,
    /// The model, from the frames.
    Judged,
}

/// One item of the checklist.
#[derive(Debug, PartialEq, Eq)]
pub struct Item {
    pub kind: Kind,
    /// Letters, digits and hyphens; unique in the checklist.
    pub id: String,
    pub text: String,
    /// Its 1-based line.
    pub line: usize,
}

/// Parse `text`, read from `file`.
///
/// # Errors
/// A list item of another shape, a repeated id, or no judged item; each
/// names `file` and the line.
pub fn parse(file: &str, text: &str) -> Result<Vec<Item>, String> {
    let mut items: Vec<Item> = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let Some(rest) = line.strip_prefix("- ") else {
            continue;
        };
        let item = item(rest, index + 1).map_err(|why| format!("{file}:{}: {why}", index + 1))?;
        if let Some(first) = items.iter().find(|i| i.id == item.id) {
            return Err(format!(
                "{file}:{}: the id `{}` is already used on line {}",
                item.line, item.id, first.line
            ));
        }
        items.push(item);
    }
    if !items.iter().any(|i| i.kind == Kind::Judged) {
        return Err(format!(
            "{file}: no `- judged <id>: <text>` item, so there is nothing to ask the model"
        ));
    }
    Ok(items)
}

fn item(rest: &str, line: usize) -> Result<Item, String> {
    let shape = "a list item is `- measured <id>: <text>` or `- judged <id>: <text>`";
    let (kind, rest) = match rest.split_once(' ') {
        Some(("measured", rest)) => (Kind::Measured, rest),
        Some(("judged", rest)) => (Kind::Judged, rest),
        _ => return Err(format!("{shape}, not `- {rest}`")),
    };
    let (id, text) = rest
        .split_once(": ")
        .ok_or_else(|| format!("{shape}; this one has no `: `"))?;
    let fits = !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    if !fits || text.trim().is_empty() {
        return Err(format!(
            "{shape}; the id `{id}` must be letters, digits and hyphens, and the text not empty"
        ));
    }
    Ok(Item {
        kind,
        id: id.to_owned(),
        text: text.trim().to_owned(),
        line,
    })
}

#[cfg(test)]
#[path = "ux_checklist_tests.rs"]
mod tests;
