//! `--samples 1|4` beside the other flags of the command line
//! (`args.rs`): the samples the full run's painters draw at
//! (`docs/plans/flat-scene-f7.md`, F7b).

use vapors_renderer::Samples;

use crate::args::USAGE;

/// The samples `value`, `--samples`' value, names.
///
/// # Errors
///
/// No value, or one other than 1 or 4, naming it and the usage.
pub(crate) fn samples_of(value: Option<&str>) -> Result<Samples, String> {
    match value {
        Some("1") => Ok(Samples::One),
        Some("4") => Ok(Samples::Four),
        Some(other) => Err(format!(
            "--samples {other}: the painter draws at 1 or 4: {USAGE}"
        )),
        None => Err(format!("--samples needs 1 or 4: {USAGE}")),
    }
}

/// `--samples` picks how the full run or a scene run draws, which the
/// other modes replace.
///
/// # Errors
///
/// `--samples` given with one of `modes`, naming it.
pub(crate) fn refuse_samples_with_modes(
    samples: Option<Samples>,
    modes: [(&str, bool); 5],
) -> Result<(), String> {
    match modes.iter().find(|(_, given)| *given) {
        Some((mode, _)) if samples.is_some() => {
            Err(format!("--samples and {mode} cannot be combined: {USAGE}"))
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
#[path = "args_samples_tests.rs"]
mod tests;
