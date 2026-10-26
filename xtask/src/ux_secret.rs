//! The API key ux-review sends, and the guard every output passes
//! through: the report, the scorecard, the PR comment, stdout and stderr.

/// The length of a run of the key that no output may hold. An API error
/// may echo the key's end (`OpenAI` shows its last four characters); twelve
/// characters of a random key are as identifying as the whole.
pub const RUN: usize = 12;

/// A key that is never printed: its `Debug` shows no part of it.
pub struct Secret {
    key: String,
    /// Every [`RUN`]-character window of the key and of its base64.
    needles: Vec<String>,
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret(..)")
    }
}

impl Secret {
    /// Keep `key`, refused when it is too short for a run of [`RUN`] to
    /// identify it, or not ASCII.
    ///
    /// # Errors
    /// Either, each with its own message; neither shows any of the key.
    pub fn new(key: String) -> Result<Self, String> {
        if !key.is_ascii() {
            return Err("the API key is not ASCII, so it is not a key this guard knows; fix: check the key's variable".to_owned());
        }
        if key.len() < 2 * RUN {
            return Err(format!(
                "the API key is {} characters, under {}, too short to guard by runs of {RUN}; fix: check the key's variable holds the whole key",
                key.len(),
                2 * RUN
            ));
        }
        let needles = needles(&key);
        Ok(Self { key, needles })
    }

    /// `text`, when it holds no run of the key, as written, with its
    /// whitespace removed, or in base64; `output` names where it was going.
    ///
    /// # Errors
    /// A run found; the refusal names `output` and shows none of the key.
    pub fn guard<'a>(&self, output: &str, text: &'a str) -> Result<&'a str, String> {
        let squeezed: String = text.chars().filter(|c| !c.is_ascii_whitespace()).collect();
        let leaks = self
            .needles
            .iter()
            .any(|needle| text.contains(needle.as_str()) || squeezed.contains(needle.as_str()));
        if leaks {
            return Err(format!(
                "{output} would hold part of the API key, so it was not written; fix: find what copied the key into it"
            ));
        }
        Ok(text)
    }
}

/// The key's [`RUN`]-character windows, and those of its standard and
/// URL-safe base64 at each of the three alignments it can take inside a
/// longer encoding (Basic auth's `user:key`, or an echoed body).
fn needles(key: &str) -> Vec<String> {
    let mut texts = vec![key.to_owned()];
    for offset in 0..3 {
        let mut bytes = vec![0u8; offset];
        bytes.extend_from_slice(key.as_bytes());
        let encoded = crate::ux_base64::encode(&bytes);
        // The characters that come from the key's bytes alone.
        let inner = encoded[(8 * offset).div_ceil(6)..8 * (offset + key.len()) / 6].to_owned();
        texts.push(inner.replace('+', "-").replace('/', "_"));
        texts.push(inner);
    }
    texts
        .iter()
        .flat_map(|t| (0..=t.len() - RUN).map(move |at| t[at..at + RUN].to_owned()))
        .collect()
}

#[cfg(test)]
#[path = "ux_secret_tests.rs"]
mod tests;
