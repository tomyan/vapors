//! What a session's API calls cost (`docs/plans/api-builder.md`): each
//! model's price per million tokens, from `[api_build.prices]` in
//! `.config/factory.toml`, as `model = "input output cache_write
//! cache_read"` in US dollars. A model with no price is refused, so a
//! budget is never counted with another model's numbers.

use std::collections::BTreeMap;

use crate::config::Config;

/// One model's dollars per million tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Price {
    pub input: f64,
    pub output: f64,
    /// A five-minute cache write, the CLI's.
    pub cache_write: f64,
    pub cache_read: f64,
}

/// The tokens one response counted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Usage {
    pub input: u64,
    pub output: u64,
    pub cache_write: u64,
    pub cache_read: u64,
}

impl Price {
    /// The most `demand` can cost: every output token it allows, and as
    /// many input tokens as its body has bytes, each at the dearer of a
    /// plain input token and a cache write.
    #[allow(clippy::cast_precision_loss)] // token counts far below 2^52
    pub fn worst(&self, demand: &crate::api_request_policy::Demand) -> f64 {
        let input = self.input.max(self.cache_write);
        (demand.max_tokens as f64 * self.output + demand.input_bound as f64 * input) / 1_000_000.0
    }

    /// What `usage` costs, in dollars.
    #[allow(clippy::cast_precision_loss)] // token counts far below 2^52
    pub fn of(&self, usage: Usage) -> f64 {
        let dollars = |tokens: u64, per_million: f64| tokens as f64 * per_million / 1_000_000.0;
        dollars(usage.input, self.input)
            + dollars(usage.output, self.output)
            + dollars(usage.cache_write, self.cache_write)
            + dollars(usage.cache_read, self.cache_read)
    }
}

/// The prices in `config`, by model.
///
/// # Errors
///
/// An entry that is not four non-negative numbers, naming it.
pub fn prices(config: &Config) -> Result<BTreeMap<String, Price>, String> {
    let entries = config.entries_under("api_build.prices.");
    entries
        .into_iter()
        .map(|(model, value)| Ok((model.clone(), price(&model, &value)?)))
        .collect()
}

fn price(model: &str, value: &str) -> Result<Price, String> {
    let numbers: Vec<f64> = value
        .split_whitespace()
        .map(str::parse::<f64>)
        .collect::<Result<_, _>>()
        .map_err(|_| shape(model, value))?;
    match numbers[..] {
        [input, output, cache_write, cache_read] if numbers.iter().all(|n| *n >= 0.0) => {
            Ok(Price {
                input,
                output,
                cache_write,
                cache_read,
            })
        }
        _ => Err(shape(model, value)),
    }
}

fn shape(model: &str, value: &str) -> String {
    format!(
        "[api_build.prices] {model} = {value:?} in .config/factory.toml: write four dollar \
         amounts per million tokens, \"input output cache_write cache_read\""
    )
}

#[cfg(test)]
#[path = "api_prices_tests.rs"]
mod tests;
