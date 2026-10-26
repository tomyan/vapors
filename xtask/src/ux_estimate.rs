//! What a review would cost before it is sent: input tokens estimated
//! from above, and the worst case with every output token used. A run
//! over either cap is refused before any request.

use crate::ux_config::{Caps, Prices};

/// Text: one token per 3 bytes, rounded up. English runs near 4 bytes a
/// token, so this overestimates.
pub fn text_tokens(bytes: usize) -> u64 {
    (bytes as u64).div_ceil(3)
}

/// One image at `detail`, the larger of `OpenAI`'s two published counts
/// (developers.openai.com/api/docs/guides/images-vision, read
/// 2026-10-26), so the estimate is above either:
/// - tiles: once the image fits 2048 px and its short side is at most
///   768 px, a base plus a count per 512 px tile; the largest listed
///   for a current model is 85 and 170 (`gpt-5` is 70 and 140), and
///   "low" is the base alone;
/// - patches: one per 32 px square, at most 2,500 at "high", times the
///   largest listed multiplier for a current model, 1.2; "low" fits the
///   image within 512 px.
pub fn image_tokens((width, height): (u32, u32), detail: &str) -> u64 {
    if detail == "low" {
        let fit = 512.0 / f64::from(width.max(height));
        let scale = |n: u32| whole((f64::from(n) * fit.min(1.0)).ceil());
        return 85_u64.max(patch_tokens(scale(width), scale(height)));
    }
    tiles(width, height).max(patch_tokens(width.into(), height.into()))
}

fn tiles(width: u32, height: u32) -> u64 {
    let (w, h) = (f64::from(width), f64::from(height));
    let fit = (2048.0 / w.max(h)).min(1.0);
    let (w, h) = (w * fit, h * fit);
    let short = (768.0 / w.min(h)).min(1.0);
    let tiles = ((w * short) / 512.0).ceil() * ((h * short) / 512.0).ceil();
    85 + 170 * whole(tiles)
}

/// Patch tokens: the count, at most the 2,500 budget, times 1.2 rounded
/// up (as tenths, to stay in whole numbers).
fn patch_tokens(width: u64, height: u64) -> u64 {
    let patches = (width.div_ceil(32) * height.div_ceil(32)).min(2500);
    (patches * 12).div_ceil(10)
}

/// `n`, a non-negative whole number (tiles, or pixels of a scaled
/// side), as a count.
fn whole(n: f64) -> u64 {
    // Tiles of an image under 2048 px each way are at most 16, and a side
    // fitted to 512 px at most 512.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a small non-negative whole number"
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
