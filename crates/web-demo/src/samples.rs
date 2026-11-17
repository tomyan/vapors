//! `?samples=1|4`: the page drawn at that sample count, its GPU time
//! written to the status line once a second, for a run by hand
//! (`docs/plans/flat-scene-f7.md`, F7b). In every build, as `?demo=` and
//! `?ticks=` are: a debug build would time debug code.

use vapors_renderer::Samples;

/// The samples `value`, the `samples` parameter, asks for; `None` when it
/// is absent.
///
/// # Errors
///
/// A value other than 1 or 4, naming it and the fix.
pub fn samples_of(value: Option<&str>) -> Result<Option<Samples>, String> {
    match value {
        None => Ok(None),
        Some("1") => Ok(Some(Samples::One)),
        Some("4") => Ok(Some(Samples::Four)),
        Some(other) => Err(format!(
            "samples={other}: the page draws at 1 or 4 samples; use ?samples=1 or ?samples=4, or leave it out"
        )),
    }
}

#[cfg(test)]
mod tests {
    use vapors_renderer::Samples;

    use super::samples_of;

    #[test]
    fn the_page_draws_at_the_samples_asked_or_as_before() {
        assert_eq!(samples_of(None), Ok(None));
        assert_eq!(samples_of(Some("1")), Ok(Some(Samples::One)));
        assert_eq!(samples_of(Some("4")), Ok(Some(Samples::Four)));
    }

    #[test]
    fn another_value_is_refused_naming_it() {
        let refused = samples_of(Some("8")).expect_err("refused");
        assert!(
            refused.starts_with("samples=8: the page draws at 1 or 4"),
            "{refused}"
        );
    }
}
