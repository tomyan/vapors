//! The pinned tool list in `.config/tools.toml`.
//!
//! The file is two flat tables, `[tools]` (crate name to version) and
//! `[binaries]` (crate name to binary name, for crates whose binary is
//! named differently). Any other shape is refused with the line number.

use std::fmt;

/// One pinned tool.
#[derive(Debug, PartialEq, Eq)]
pub struct Tool {
    /// The crate name, as `cargo binstall` takes it.
    pub crate_name: String,
    /// The binary that answers `--version`.
    pub binary: String,
    /// The exact version required.
    pub version: String,
}

/// Why a tools file could not be read.
#[derive(Debug, PartialEq, Eq)]
pub struct ParseError {
    /// 1-based line number.
    pub line: usize,
    /// What was wrong with it.
    pub reason: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, ".config/tools.toml:{}: {}", self.line, self.reason)
    }
}

enum Table {
    None,
    Tools,
    Binaries,
}

/// Parse the tools file. Refuses anything that is not a comment, a
/// blank line, one of the two table headers, or `key = "value"`.
pub fn parse(text: &str) -> Result<Vec<Tool>, ParseError> {
    let mut table = Table::None;
    let mut tools: Vec<Tool> = Vec::new();
    let mut binaries: Vec<(String, String)> = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        let number = index + 1;
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match line {
            "[tools]" => table = Table::Tools,
            "[binaries]" => table = Table::Binaries,
            _ => {
                let (key, value) = key_value(line, number)?;
                match table {
                    Table::Tools => tools.push(Tool {
                        binary: key.clone(),
                        crate_name: key,
                        version: value,
                    }),
                    Table::Binaries => binaries.push((key, value)),
                    Table::None => {
                        return Err(ParseError {
                            line: number,
                            reason: "entry before any [tools] or [binaries] header".into(),
                        });
                    }
                }
            }
        }
    }
    apply_binaries(&mut tools, &binaries)?;
    Ok(tools)
}

fn key_value(line: &str, number: usize) -> Result<(String, String), ParseError> {
    let refuse = |reason: &str| ParseError {
        line: number,
        reason: format!("{reason}: `{line}`"),
    };
    let (key, rest) = line
        .split_once('=')
        .ok_or_else(|| refuse("expected `key = \"value\"`"))?;
    let value = rest
        .trim()
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .ok_or_else(|| refuse("value must be a double-quoted string"))?;
    if key.trim().is_empty() || value.is_empty() {
        return Err(refuse("key and value must both be non-empty"));
    }
    Ok((key.trim().to_owned(), value.to_owned()))
}

fn apply_binaries(tools: &mut [Tool], binaries: &[(String, String)]) -> Result<(), ParseError> {
    for (crate_name, binary) in binaries {
        let tool = tools
            .iter_mut()
            .find(|t| &t.crate_name == crate_name)
            .ok_or_else(|| ParseError {
                line: 0,
                reason: format!("[binaries] names `{crate_name}`, which is not in [tools]"),
            })?;
        tool.binary.clone_from(binary);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tools_and_binary_overrides() {
        // Given
        let text = "# pinned\n[tools]\ncargo-nextest = \"0.9.1\"\nwasm-bindgen-cli = \"0.2.1\"\n\n[binaries]\nwasm-bindgen-cli = \"wasm-bindgen\"\n";

        // When
        let tools = parse(text).expect("valid file");

        // Then
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].binary, "cargo-nextest");
        assert_eq!(tools[1].binary, "wasm-bindgen");
        assert_eq!(tools[1].version, "0.2.1");
    }

    #[test]
    fn refuses_unquoted_value_naming_the_line() {
        // Given
        let text = "[tools]\nbacon = 3.26\n";

        // When
        let error = parse(text).expect_err("unquoted value");

        // Then
        assert_eq!(error.line, 2);
        assert!(error.reason.contains("double-quoted"));
    }

    #[test]
    fn refuses_entry_before_a_table_header() {
        // Given
        let text = "bacon = \"3.26.0\"\n";

        // When
        let error = parse(text).expect_err("no header");

        // Then
        assert_eq!(error.line, 1);
    }

    #[test]
    fn refuses_binary_override_for_unknown_tool() {
        // Given
        let text = "[tools]\nbacon = \"1\"\n[binaries]\ntrunk = \"trunk\"\n";

        // When
        let error = parse(text).expect_err("unknown tool");

        // Then
        assert!(error.reason.contains("`trunk`"));
    }
}
