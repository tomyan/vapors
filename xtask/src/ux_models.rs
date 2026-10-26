//! How each model counts an image's input tokens, as `OpenAI` publishes it.
//! A model with no row here is refused (`ux_config`), so a review's
//! estimate is never made with another model's numbers.

/// Where the rows came from.
pub const GUIDE: &str = "developers.openai.com/api/docs/guides/images-vision, read 2026-10-26";

/// How a model sizes an image at one detail level before counting
/// patches: fitted so its longer side is at most `max_side`, then at most
/// `budget` patches of 32 px (`None`: no budget below the 30,000-patch
/// rejection limit).
#[derive(Debug, PartialEq, Eq)]
pub struct Fit {
    pub max_side: u32,
    pub budget: Option<u64>,
}

/// A model's image counting.
#[derive(Debug, PartialEq, Eq)]
pub enum Images {
    /// A base plus a count per 512 px tile, once the image fits 2048 px
    /// and its short side is at most 768 px; "low" is the base alone.
    Tiles { base: u64, per_tile: u64 },
    /// 32 px patches, times `tenths` / 10, rounded up.
    Patches { tenths: u64, low: Fit, high: Fit },
}

/// One model's row.
#[derive(Debug, PartialEq, Eq)]
pub struct Model {
    pub name: &'static str,
    pub images: Images,
}

const fn tiles(name: &'static str, base: u64, per_tile: u64) -> Model {
    Model {
        name,
        images: Images::Tiles { base, per_tile },
    }
}

const fn patches(name: &'static str, low: Fit, high: Fit) -> Model {
    Model {
        name,
        images: Images::Patches {
            tenths: 12,
            low,
            high,
        },
    }
}

const fn fit(max_side: u32, budget: Option<u64>) -> Fit {
    Fit { max_side, budget }
}

/// The models the guide lists, with its numbers. The guide gives a
/// multiplier of 1.2 for every patch-counted model here.
const MODELS: [Model; 14] = [
    tiles("gpt-4o", 85, 170),
    tiles("gpt-4.1", 85, 170),
    tiles("gpt-4o-mini", 2833, 5667),
    tiles("gpt-5", 70, 140),
    tiles("gpt-5.1", 70, 140),
    patches("gpt-5.2", fit(2048, Some(6144)), fit(2048, Some(6144))),
    patches("gpt-5.4", fit(2048, Some(6144)), fit(2048, Some(2500))),
    patches("gpt-5.4-mini", fit(2048, Some(6144)), fit(2048, Some(2500))),
    patches("gpt-5.4-nano", fit(2048, Some(6144)), fit(2048, Some(2500))),
    patches("gpt-5.5", fit(512, None), fit(2048, Some(2500))),
    patches("gpt-5.6-sol", fit(512, None), fit(2048, Some(2500))),
    patches("gpt-5.6-terra", fit(512, None), fit(2048, Some(2500))),
    patches("gpt-5.6-luna", fit(512, None), fit(2048, Some(2500))),
    patches("gpt-6-astra", fit(512, None), fit(65_535, Some(2500))),
];

/// The row for `name`.
///
/// # Errors
/// No row, naming the model and the fix.
pub fn find(name: &str) -> Result<&'static Model, String> {
    MODELS.iter().find(|m| m.name == name).ok_or_else(|| {
        format!(
            "ux.model `{name}` has no image-token row in xtask/src/ux_models.rs, so its cost cannot be bounded; fix: add its numbers from {GUIDE} (or a dated measurement), or choose a listed model"
        )
    })
}
