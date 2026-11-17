//! `--size <width>x<height>` and `--profile-only` beside `--scene-run`
//! (`args.rs`): a scene run at another size than its own, and one that
//! reports without the budget's verdict, on any adapter, a software one
//! included (queue item 13b, docs/perf/render-profile-13b.md).

use crate::args::USAGE;

/// The largest side `--size` takes: wgpu's default
/// `max_texture_dimension_2d`, which every adapter vapors runs on has.
const LARGEST_SIDE: u32 = 8192;

/// How a scene run runs, beyond its scene and its samples.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SceneProfile {
    /// The frame's size in pixels, in place of the scene's own.
    pub size: Option<(u32, u32)>,
    /// Report every frame without judging the run against the frame
    /// budget, on any adapter; a busy machine is still refused.
    pub profile_only: bool,
}

/// The size `value`, `--size`'s value, names.
///
/// # Errors
/// No value, or one that is not two whole numbers from 1 to 8192 joined
/// by `x`, naming it and the usage.
pub(crate) fn size_of(value: Option<&str>) -> Result<(u32, u32), String> {
    let side = |text: &str| {
        text.parse::<u32>()
            .ok()
            .filter(|s| (1..=LARGEST_SIDE).contains(s))
    };
    let parsed = value
        .and_then(|value| value.split_once('x'))
        .and_then(|(width, height)| Some((side(width)?, side(height)?)));
    parsed.ok_or_else(|| {
        format!(
            "--size {}: give <width>x<height> in pixels, each 1 to {LARGEST_SIDE}, such as \
             1920x1080: {USAGE}",
            value.unwrap_or("(no value)")
        )
    })
}

/// `--size` and `--profile-only` change a scene run, so each needs one.
///
/// # Errors
/// Either given without `--scene-run`, naming it.
pub(crate) fn refuse_without_scene_run(
    profile: SceneProfile,
    scene_run: bool,
) -> Result<(), String> {
    let given = [
        ("--size", profile.size.is_some()),
        ("--profile-only", profile.profile_only),
    ];
    match given.iter().find(|(_, given)| *given) {
        Some((flag, _)) if !scene_run => Err(format!(
            "{flag} changes a scene run; give it with --scene-run: {USAGE}"
        )),
        _ => Ok(()),
    }
}

#[cfg(test)]
#[path = "args_scene_profile_tests.rs"]
mod tests;
