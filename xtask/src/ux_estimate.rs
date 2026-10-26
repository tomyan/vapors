//! What a review would cost before it is sent: input tokens bounded
//! from above, and the worst case with every output token used. A run
//! over either cap is refused before any request.

use crate::ux_config::{Caps, Prices};
use crate::ux_models::{Fit, Images, Model};

/// Text: one token per byte. A token covers at least one byte, so this
/// is an upper bound without the tokenizer.
pub fn text_tokens(bytes: usize) -> u64 {
    bytes as u64
}

/// One image of `size` at `detail`, counted with `model`'s row
/// (`ux_models`).
pub fn image_tokens(model: &Model, size: (u32, u32), detail: &str) -> u64 {
    match &model.images {
        Images::Tiles { base, .. } if detail == "low" => *base,
        Images::Tiles { base, per_tile } => base + per_tile * tile_count(size),
        Images::Patches { tenths, low, high } => {
            let fit = if detail == "low" { low } else { high };
            (patch_count(size, fit) * tenths).div_ceil(10)
        }
    }
}

/// 512 px tiles once the image fits 2048 px and its short side is at
/// most 768 px.
fn tile_count((width, height): (u32, u32)) -> u64 {
    let (w, h) = (f64::from(width), f64::from(height));
    let fit = (2048.0 / w.max(h)).min(1.0);
    let (w, h) = (w * fit, h * fit);
    let short = (768.0 / w.min(h)).min(1.0);
    whole(((w * short) / 512.0).ceil()) * whole(((h * short) / 512.0).ceil())
}

/// 32 px patches once the longer side fits `fit.max_side`, at most its
/// budget: an image over the budget is scaled down to it.
fn patch_count((width, height): (u32, u32), fit: &Fit) -> u64 {
    let scale = (f64::from(fit.max_side) / f64::from(width.max(height))).min(1.0);
    let side = |n: u32| whole((f64::from(n) * scale).ceil());
    let raw = side(width).div_ceil(32) * side(height).div_ceil(32);
    fit.budget.map_or(raw, |budget| raw.min(budget))
}

/// `n`, a non-negative whole number (tiles per side, or pixels of a
/// fitted side, at most 65,535), as a count.
fn whole(n: f64) -> u64 {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a non-negative whole number under 65,536"
    )]
    let count = n as u64;
    count
}

/// A request's estimated input.
#[derive(Debug, PartialEq, Eq)]
pub struct Estimate {
    pub text: u64,
    pub images: u64,
}

impl Estimate {
    pub fn input(&self) -> u64 {
        self.text + self.images
    }

    /// The most the request can cost: its estimated input, and every
    /// output token the cap allows, at `prices` per million.
    pub fn worst_cost(&self, caps: &Caps, prices: &Prices) -> f64 {
        dollars(self.input(), prices.input) + dollars(caps.output_tokens, prices.output)
    }

    /// Refuse a request over the input cap or the per-run cost cap.
    ///
    /// # Errors
    /// Either cap exceeded, with both numbers and the fix.
    pub fn within(&self, caps: &Caps, prices: &Prices) -> Result<(), String> {
        if self.input() > caps.input_tokens {
            return Err(format!(
                "the request is estimated at {} input tokens, over ux.max_input_tokens {}; fix: send fewer frames or design files",
                self.input(),
                caps.input_tokens
            ));
        }
        let worst = self.worst_cost(caps, prices);
        if worst > caps.cost_usd {
            return Err(format!(
                "the request could cost ${worst:.4}, over ux.max_cost_usd ${:.2}; fix: lower ux.max_output_tokens or send less",
                caps.cost_usd
            ));
        }
        Ok(())
    }
}

/// The cost of `tokens` at `per_million` dollars.
pub fn dollars(tokens: u64, per_million: f64) -> f64 {
    // Token counts stay far under 2^52, where f64 is exact.
    #[allow(clippy::cast_precision_loss, reason = "counts far under 2^52")]
    let tokens = tokens as f64;
    tokens * per_million / 1_000_000.0
}

#[cfg(test)]
#[path = "ux_estimate_tests.rs"]
mod tests;
