//! Evaluating a parsed media feature against the environment or a
//! container's size.

use super::Environment;
use super::media_feature::{Feature, SizeAxis};
use crate::value::{ColorScheme, Context};

/// What size features compare against: the viewport or a container.
pub(crate) struct Sizes {
    width: f64,
    height: f64,
    resolution: f64,
    color_scheme: ColorScheme,
    reduced_motion: bool,
    /// What feature lengths resolve against.
    ctx: Context,
}

impl Sizes {
    pub(crate) fn viewport(env: &Environment) -> Self {
        Self::container(env.viewport_width, env.viewport_height, env)
    }

    #[allow(clippy::cast_possible_truncation)] // f64 holds any f32 exactly
    pub(crate) fn container(
        width: impl Into<f64>,
        height: impl Into<f64>,
        env: &Environment,
    ) -> Self {
        Self {
            width: width.into(),
            height: height.into(),
            resolution: env.resolution,
            color_scheme: env.color_scheme,
            reduced_motion: env.reduced_motion,
            ctx: Context {
                font_size: env.root_font_size,
                root_font_size: env.root_font_size,
                viewport_width: env.viewport_width,
                viewport_height: env.viewport_height,
                zero_advance: env.root_font_size * env.zero_advance_em,
                percent_base: None,
            },
        }
    }
}

impl Feature {
    pub(crate) fn holds(&self, sizes: &Sizes) -> bool {
        match self {
            Self::Size(axis, tests) => {
                let actual = match axis {
                    SizeAxis::Width => sizes.width,
                    SizeAxis::Height => sizes.height,
                };
                tests
                    .iter()
                    .all(|(op, l)| l.to_px(&sizes.ctx).is_ok_and(|v| op.holds(actual, v)))
            }
            Self::Resolution(tests) => tests.iter().all(|(op, v)| op.holds(sizes.resolution, *v)),
            Self::ColorScheme(s) => sizes.color_scheme == *s,
            Self::ReducedMotion(r) => sizes.reduced_motion == *r,
        }
    }
}
