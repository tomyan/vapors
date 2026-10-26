//! The API key ux-review sends, and the guard every output passes
//! through: the report, the scorecard, the PR comment, stdout and stderr.

/// The length of a run of the key that no output may hold. An API error
/// may echo the key's end (`OpenAI` shows its last four characters); twelve
/// characters of a random key are as identifying as the whole.
pub const RUN: usize = 12;

/// A key that is never printed: its `Debug` shows no part of it.
pub struct Secret(String);

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret(..)")
    }
}

impl Secret {
    /// Keep `key`, refused when it is too short for a run of [`RUN`] to
    /// identify it.
    ///
    /// # Errors
    /// A key under twice [`RUN`] characters, or not ASCII; the refusal
    /// shows no part of it.
    pub fn new(key: String) -> Result<Self, String> {
        if key.len() < 2 * RUN || !key.is_ascii() {
            return Err(format!(
                "the API key is too short to guard ({} characters, under {}) or not ASCII; fix: check the key's variable holds the whole key",
                key.len(),
                2 * RUN
            ));
        }
        Ok(Self(key))
    }

    /// `text`, when it holds no run of the key; `output` names where it
    /// was going.
    ///
    /// # Errors
    /// `text` holds [`RUN`] or more characters of the key in a row; the
    /// refusal names `output` and shows no part of the key.
    pub fn guard<'a>(&self, output: &str, text: &'a str) -> Result<&'a str, String> {
        let leaks = (0..=self.0.len() - RUN).any(|at| text.contains(&self.0[at..at + RUN]));
        if leaks {
            return Err(format!(
                "{output} would hold part of the API key, so it was not written; fix: find what copied the key into it"
            ));
        }
        Ok(text)
    }
}

#[cfg(test)]
#[path = "ux_secret_tests.rs"]
mod tests;
