//! Text a push publishes under the owner's account: the PR's title and
//! body, and the pushed commits' messages and author and committer names
//! (review of slice 4, L3; verify V2). GitHub notifies anyone @-mentioned
//! and closes an issue a closing keyword names on merge, a squash merge's
//! message is composed from the commits', and a control byte reaches the
//! terminal of whoever reads the PR or `git log`. Each is refused, named.

/// The words GitHub reads as closing an issue when followed by a
/// reference.
const CLOSING: [&str; 9] = [
    "close", "closes", "closed", "fix", "fixes", "fixed", "resolve", "resolves", "resolved",
];

/// Refuses `text` (the `what` of a push) holding a control byte other
/// than newline and tab, an @-mention anywhere, or a closing keyword
/// before an issue reference in any of GitHub's forms.
///
/// # Errors
///
/// The first such, named.
pub fn published(what: &str, text: &str) -> Result<(), String> {
    if let Some(c) = text
        .chars()
        .find(|&c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(format!(
            "{what} holds the control character {c:?}: send text only"
        ));
    }
    if let Some(mention) = mention(text) {
        return Err(format!(
            "{what} mentions {mention:?}: GitHub would notify them; leave out the @"
        ));
    }
    let words: Vec<&str> = text.split_whitespace().collect();
    let closes = words
        .windows(2)
        .find(|pair| closing(pair[0]) && reference(pair[1]));
    match closes {
        Some(pair) => Err(format!(
            "{what} says {:?}, which closes an issue on merge: the lead links issues",
            pair.join(" ")
        )),
        None => Ok(()),
    }
}

/// The first `@name` in `text`: an `@` not inside an email address
/// (after an ASCII letter, digit, `_`, `.`, `+` or `-`) and followed by
/// a letter, digit or `-`. ASCII only, as GitHub's mention filter reads a
/// word boundary (`\W`): `é@name` is a mention there (re-verify, R1).
fn mention(text: &str) -> Option<&str> {
    text.match_indices('@').find_map(|(at, _)| {
        let after = &text[at + 1..];
        let inside_word = text[..at]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_ascii_alphanumeric() || "_.+-".contains(c));
        let named = after.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '-');
        (named && !inside_word).then(|| &text[at..text.len().min(at + 20)])
    })
}

fn closing(word: &str) -> bool {
    let word = word
        .trim_start_matches(['(', '[', '"', '\''])
        .trim_end_matches([':', ',', '.']);
    CLOSING.contains(&word.to_lowercase().as_str())
}

/// An issue reference: `#N`, `owner/repo#N`, `GH-N`, or an issue or PR
/// URL.
fn reference(word: &str) -> bool {
    let word = word
        .trim_start_matches(['(', '['])
        .trim_end_matches([')', ']', '.', ',', ';', ':']);
    let numbered = |after: &str| after.starts_with(|c: char| c.is_ascii_digit());
    let hash = word.find('#').is_some_and(|at| numbered(&word[at + 1..]));
    let gh = word
        .to_lowercase()
        .strip_prefix("gh-")
        .is_some_and(numbered);
    let url = ["/issues/", "/pull/"]
        .iter()
        .any(|path| word.contains(path));
    hash || gh || url
}

#[cfg(test)]
#[path = "api_push_text_tests.rs"]
mod tests;
