//! `[ux]` and `[ux.design]` in factory.toml: what `ux-review` reads, the
//! model it asks, its prices and its caps. Every value is checked here,
//! so a review never starts on a guessed number.

use std::path::PathBuf;

use crate::config::Config;

/// Per million tokens, in US dollars, and where they were read.
#[derive(Debug, PartialEq)]
pub struct Prices {
    pub input: f64,
    pub cached: f64,
    pub output: f64,
    /// The page and the date the prices were read on.
    pub source: String,
}

/// What one run, and one day, may spend.
#[derive(Debug, PartialEq)]
pub struct Caps {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
    pub daily_usd: f64,
}

/// The review's settings.
#[derive(Debug, PartialEq)]
pub struct UxConfig {
    /// The walk's run directory: `<viewport>/<nn>-<step>.png` and
    /// `facts.json`.
    pub run_dir: PathBuf,
    pub checklist: PathBuf,
    /// The design files sent with the frames.
    pub design: Vec<PathBuf>,
    /// Reports, the scorecard and the spend ledger.
    pub report_dir: PathBuf,
    pub model: String,
    /// The Responses API URL.
    pub endpoint: String,
    /// The environment variable holding the API key.
    pub key_env: String,
    pub reasoning: String,
    pub image_detail: String,
    pub prices: Prices,
    pub caps: Caps,
}

impl UxConfig {
    /// Read `[ux]` and `[ux.design]`; `home` expands a leading `~/`.
    ///
    /// # Errors
    /// A missing key, a number that is not positive and finite, a price
    /// source with no date, or a value outside its listed choices.
    pub fn from_config(config: &Config, home: Option<&str>) -> Result<Self, String> {
        let text = |key: &str| {
            config
                .require(&format!("ux.{key}"))
                .map(str::to_owned)
                .map_err(|e| e.to_string())
        };
        Ok(Self {
            run_dir: PathBuf::from(text("run_dir")?),
            checklist: PathBuf::from(text("checklist")?),
            design: design(config)?,
            report_dir: expand(&text("report_dir")?, home)?,
            model: text("model")?,
            endpoint: endpoint(&text("endpoint")?)?,
            key_env: text("key_env")?,
            reasoning: one_of(
                "ux.reasoning",
                &text("reasoning")?,
                &["low", "medium", "high"],
            )?,
            image_detail: one_of("ux.image_detail", &text("image_detail")?, &["low", "high"])?,
            prices: prices(config)?,
            caps: caps(config)?,
        })
    }
}

fn design(config: &Config) -> Result<Vec<PathBuf>, String> {
    let files: Vec<PathBuf> = config
        .entries_under("ux.design.")
        .into_keys()
        .map(PathBuf::from)
        .collect();
    if files.is_empty() {
        return Err("[ux.design] lists no design files; fix: add `\"design/patterns.md\" = \"<why it is sent>\"`".to_owned());
    }
    Ok(files)
}

fn expand(path: &str, home: Option<&str>) -> Result<PathBuf, String> {
    let Some(rest) = path.strip_prefix("~/") else {
        return Ok(PathBuf::from(path));
    };
    let home = home.ok_or(
        "ux.report_dir starts with ~/ but HOME is not set; fix: set HOME or give a full path",
    )?;
    Ok(PathBuf::from(home).join(rest))
}

/// HTTPS, or a local stand-in for tests.
fn endpoint(url: &str) -> Result<String, String> {
    if url.starts_with("https://") || url.starts_with("http://127.0.0.1:") {
        return Ok(url.to_owned());
    }
    Err(format!(
        "ux.endpoint `{url}` is neither https:// nor a local stand-in at http://127.0.0.1:<port>; fix: use the provider's https URL"
    ))
}

fn one_of(key: &str, value: &str, choices: &[&str]) -> Result<String, String> {
    if choices.contains(&value) {
        return Ok(value.to_owned());
    }
    Err(format!("{key} is `{value}`, not one of {choices:?}"))
}

fn number(config: &Config, key: &str) -> Result<f64, String> {
    let text = config.require(key).map_err(|e| e.to_string())?;
    match text.parse::<f64>() {
        Ok(n) if n.is_finite() && n > 0.0 => Ok(n),
        _ => Err(format!("{key} is `{text}`, not a positive number")),
    }
}

fn count(config: &Config, key: &str) -> Result<u64, String> {
    let text = config.require(key).map_err(|e| e.to_string())?;
    match text.parse::<u64>() {
        Ok(n) if n > 0 => Ok(n),
        _ => Err(format!("{key} is `{text}`, not a positive whole number")),
    }
}

fn prices(config: &Config) -> Result<Prices, String> {
    let source = config
        .require("ux.price_source")
        .map_err(|e| e.to_string())?;
    if !has_date(source) {
        return Err(format!(
            "ux.price_source `{source}` has no YYYY-MM-DD date; fix: say when the prices were read"
        ));
    }
    Ok(Prices {
        input: number(config, "ux.price_input")?,
        cached: number(config, "ux.price_cached")?,
        output: number(config, "ux.price_output")?,
        source: source.to_owned(),
    })
}

fn has_date(text: &str) -> bool {
    text.as_bytes().windows(10).any(|w| {
        w.iter().enumerate().all(|(i, b)| {
            if i == 4 || i == 7 {
                *b == b'-'
            } else {
                b.is_ascii_digit()
            }
        })
    })
}

fn caps(config: &Config) -> Result<Caps, String> {
    Ok(Caps {
        input_tokens: count(config, "ux.max_input_tokens")?,
        output_tokens: count(config, "ux.max_output_tokens")?,
        cost_usd: number(config, "ux.max_cost_usd")?,
        daily_usd: number(config, "ux.daily_cost_usd")?,
    })
}

#[cfg(test)]
#[path = "ux_config_tests.rs"]
mod tests;
