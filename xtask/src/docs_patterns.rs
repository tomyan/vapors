//! The `patterns` rule: text in "double quotes" in the patterns file is
//! exact UI text, and must be in the page's source. `<name>`, `…` and
//! `...` stand for variable parts; each fixed piece between them,
//! trimmed of the punctuation that joins a composed message's parts,
//! must appear in the page's source. Examples go in 'single quotes' or
//! italics, which the rule leaves alone, as it does code spans and fenced
//! code. Quotes pair within a paragraph or list item, so a quote may wrap
//! across lines; one left open is refused, naming the line it opens on.

pub use crate::docs_source_text::Source;

/// A quote and the line it opens on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quote {
    pub line: usize,
    pub text: String,
}

/// A character of a block and the line it is on.
type Placed = (char, usize);

/// The double-quoted texts of `text`, and its unpaired quotes as problems.
#[must_use]
pub fn quotes(file: &str, text: &str) -> (Vec<Quote>, Vec<String>) {
    let (mut found, mut problems) = (Vec::new(), Vec::new());
    for block in blocks(text) {
        let (quotes, open) = block_quotes(&block);
        found.extend(quotes);
        if let Some(line) = open {
            problems.push(format!(
                "{file}:{line}: a double quote is not closed in its paragraph; fix: close it, or mark an example with single quotes or italics"
            ));
        }
    }
    (found, problems)
}

/// The paragraphs and list items of `text` outside fenced code, each
/// line trimmed and joined to the next by a space.
fn blocks(text: &str) -> Vec<Vec<Placed>> {
    let (mut blocks, mut current, mut fenced) = (Vec::new(), Vec::new(), false);
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        let fence = line.starts_with("```");
        if fence || line.is_empty() || starts_block(line) {
            blocks.push(std::mem::take(&mut current));
        }
        fenced ^= fence;
        if fence || fenced || line.is_empty() {
            continue;
        }
        if !current.is_empty() {
            current.push((' ', index + 1));
        }
        current.extend(line.chars().map(|c| (c, index + 1)));
    }
    blocks.push(current);
    blocks.retain(|block| !block.is_empty());
    blocks
}

/// Whether a trimmed line begins a block of its own: a list item, a
/// heading or a table row.
fn starts_block(line: &str) -> bool {
    let numbered = line
        .split_once(". ")
        .is_some_and(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
    numbered
        || ["- ", "* ", "+ ", "#", "|"]
            .iter()
            .any(|m| line.starts_with(m))
}

/// A block's quotes outside code spans, and the line of one left open.
fn block_quotes(block: &[Placed]) -> (Vec<Quote>, Option<usize>) {
    let (mut found, mut open, mut in_code) = (Vec::new(), None::<Quote>, false);
    for &(c, line) in block {
        in_code ^= c == '`';
        if c == '`' || in_code {
            continue;
        }
        match (c, open.take()) {
            ('"', None) => {
                open = Some(Quote {
                    line,
                    text: String::new(),
                });
            }
            ('"', Some(quote)) => found.push(quote),
            (c, Some(mut quote)) => {
                quote.text.push(c);
                open = Some(quote);
            }
            (_, None) => {}
        }
    }
    (found, open.map(|quote| quote.line))
}

/// The fixed pieces of a quote, between its placeholders and ellipses.
#[must_use]
pub fn pieces(quote: &str) -> Vec<String> {
    let mut pieces = vec![String::new()];
    let mut rest = quote;
    while let Some(c) = rest.chars().next() {
        let wildcard = wildcard_len(rest);
        if wildcard > 0 {
            pieces.push(String::new());
            rest = &rest[wildcard..];
        } else {
            if let Some(piece) = pieces.last_mut() {
                piece.push(c);
            }
            rest = &rest[c.len_utf8()..];
        }
    }
    pieces
        .iter()
        .map(|piece| piece.trim_matches(|c: char| c.is_whitespace() || JOINERS.contains(c)))
        .filter(|piece| !piece.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Punctuation trimmed from the ends of a fixed piece: the page often
/// joins a message's parts with it in code (`"…: " + why + "; " + FIX`).
const JOINERS: &str = ";:,.·()[]|-";

/// The length of the placeholder or ellipsis `rest` starts with, or 0.
fn wildcard_len(rest: &str) -> usize {
    if rest.starts_with('…') {
        '…'.len_utf8()
    } else if rest.starts_with("...") {
        3
    } else if rest.starts_with('<') {
        rest.find('>').map_or(0, |end| end + 1)
    } else {
        0
    }
}

/// Whether each of `pieces` is in one of `sources`. The page composes
/// messages from literals and constants, often in different files, so
/// neither one file nor the pieces' order is required.
#[must_use]
pub fn found(pieces: &[String], sources: &[Source]) -> bool {
    pieces.iter().all(|piece| {
        sources
            .iter()
            .any(|source| source.text.contains(piece.as_str()))
    })
}

/// Every problem with `file`'s quotes against `sources`, which come from
/// `where_from` (named in the message).
#[must_use]
pub fn check(file: &str, text: &str, sources: &[Source], where_from: &str) -> Vec<String> {
    let (quotes, mut problems) = quotes(file, text);
    for quote in quotes {
        let fixed = pieces(&quote.text);
        let (line, text) = (quote.line, &quote.text);
        if fixed.is_empty() {
            problems.push(format!("{file}:{line}: \"{text}\" quotes no fixed text; fix: quote the UI text around it, or mark an example with single quotes or italics"));
        } else if !found(&fixed, sources) {
            problems.push(format!("{file}:{line}: \"{text}\" is not in the page's source ({where_from}); fix: quote the text the page shows, or mark an example with single quotes or italics"));
        }
    }
    problems
}

#[cfg(test)]
#[path = "docs_patterns_tests.rs"]
mod tests;
