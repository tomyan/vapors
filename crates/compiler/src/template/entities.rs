//! Character references in literal text and quoted attribute values:
//! the named ones HTML text needs, and decimal or hex numeric ones. An
//! `&` that does not start `&name;` is kept as written; a reference of
//! that shape that is unknown or names no character is refused.

const NAMED: &[(&str, char)] = &[
    ("amp", '&'),
    ("lt", '<'),
    ("gt", '>'),
    ("quot", '"'),
    ("apos", '\''),
    ("nbsp", '\u{a0}'),
];

/// Decodes the references in `raw`. A refusal carries the byte offset
/// of its `&` within `raw` and the message.
pub(crate) fn decode(raw: &str) -> Result<String, (usize, String)> {
    let mut decoded = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(at) = rest.find('&') {
        decoded.push_str(&rest[..at]);
        let from_amp = &rest[at..];
        let Some(len) = reference_len(from_amp) else {
            decoded.push('&');
            rest = &from_amp[1..];
            continue;
        };
        let reference = &from_amp[..len];
        let character =
            resolve(reference).ok_or_else(|| (raw.len() - from_amp.len(), refusal(reference)))?;
        decoded.push(character);
        rest = &from_amp[len..];
    }
    decoded.push_str(rest);
    Ok(decoded)
}

/// The length of `&name;` or `&#..;` at the start of `text`.
fn reference_len(text: &str) -> Option<usize> {
    let body = &text[1..];
    let end = body.find(|c: char| !(c.is_ascii_alphanumeric() || c == '#'))?;
    (end > 0 && body[end..].starts_with(';')).then_some(end + 2)
}

fn resolve(reference: &str) -> Option<char> {
    let name = &reference[1..reference.len() - 1];
    let Some(number) = name.strip_prefix('#') else {
        return NAMED
            .iter()
            .find(|(known, _)| *known == name)
            .map(|(_, character)| *character);
    };
    let value = match number.strip_prefix(['x', 'X']) {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => number.parse().ok()?,
    };
    char::from_u32(value).filter(|character| *character != '\0')
}

fn refusal(reference: &str) -> String {
    format!(
        "`{reference}` is not a character reference this compiler knows: use &amp; &lt; &gt; &quot; &apos; &nbsp;, or a character number as &#NN; or &#xNN;"
    )
}
