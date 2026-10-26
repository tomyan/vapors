//! The UX checklist (`ux.checklist`): Markdown whose list items are the
//! review's items, each `- measured <id>: <text>` or
//! `- judged <id>: <text>`. A measured item's id names a fact in the
//! walk's facts.json, filled in by the script; only judged items go to
//! the model. A list item of any other shape (nested, `*`, `+` or
//! numbered) is refused with its line; fenced code is not read.

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
    let mut fenced = false;
    for (index, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
        }
        if fenced || !list_item(line) {
            continue;
        }
        let item =
            column_item(line, index + 1).map_err(|why| format!("{file}:{}: {why}", index + 1))?;
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

/// A Markdown list item, at any indent: `- `, `* `, `+ ` or `<digits>. `.
fn list_item(line: &str) -> bool {
    let start = line.trim_start();
    let digits = start.chars().take_while(char::is_ascii_digit).count();
    ["- ", "* ", "+ "]
        .iter()
        .any(|mark| start.starts_with(mark))
        || (digits > 0 && start[digits..].starts_with(". "))
}

/// An item, which must be `- ` at column 0: a nested, starred or
/// numbered item is refused rather than left out.
fn column_item(line: &str, number: usize) -> Result<Item, String> {
    match line.strip_prefix("- ") {
        Some(rest) => item(rest, number),
        None => Err(format!(
            "a checklist item starts `- ` at column 0, not `{line}`"
        )),
    }
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
