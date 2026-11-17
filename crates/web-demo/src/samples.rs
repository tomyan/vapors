//! `?samples=1|4`: the page drawn at that sample count, its GPU time
//! written to the demo bar's line once a second, for a run by hand
//! (`docs/plans/flat-scene-f7.md`, F7b). In every build, as `?demo=` and
//! `?ticks=` are: a debug build would time debug code. Read once, in
//! `start`, and refused for a demo that does not draw at a chosen count.

use vapors_renderer::Samples;

/// The samples `values`, every `samples` parameter given, ask for;
/// `None` when there is none.
///
/// # Errors
///
/// A value other than 1 or 4, or the parameter given more than once,
/// naming the fix.
pub fn samples_of(values: &[String]) -> Result<Option<Samples>, String> {
    match values {
        [] => Ok(None),
        [one] => samples_from(one).map(Some),
        _ => Err(format!(
            "samples given {} times ({}): give ?samples= once",
            values.len(),
            values.join(", ")
        )),
    }
}

fn samples_from(value: &str) -> Result<Samples, String> {
    match value {
        "1" => Ok(Samples::One),
        "4" => Ok(Samples::Four),
        other => Err(format!(
            "samples={other}: the page draws at 1 or 4 samples; use ?samples=1 or ?samples=4, or leave it out"
        )),
    }
}

/// `samples` for demo `name`, which draws at 1 sample and shows no GPU
/// time: refused when given.
///
/// # Errors
///
/// `samples` given, naming the demo and the fix.
pub fn none_for(name: &str, samples: Option<Samples>) -> Result<(), String> {
    match samples {
        None => Ok(()),
        Some(_) => Err(format!(
            "{name} draws at 1 sample and shows no GPU time: remove samples= from the URL"
        )),
    }
}

#[cfg(test)]
mod tests {
    use vapors_renderer::Samples;

    use super::{none_for, samples_of};

    fn given(values: &[&str]) -> Result<Option<Samples>, String> {
        samples_of(&values.iter().map(ToString::to_string).collect::<Vec<_>>())
    }

    #[test]
    fn the_page_draws_at_the_samples_asked_or_as_before() {
        assert_eq!(given(&[]), Ok(None));
        assert_eq!(given(&["1"]), Ok(Some(Samples::One)));
        assert_eq!(given(&["4"]), Ok(Some(Samples::Four)));
    }

    #[test]
    fn another_value_is_refused_naming_it() {
        let refused = given(&["8"]).expect_err("refused");
        assert!(
            refused.starts_with("samples=8: the page draws at 1 or 4"),
            "{refused}"
        );
    }

    #[test]
    fn samples_given_twice_are_refused_naming_both() {
        assert_eq!(
            given(&["4", "1"]),
            Err("samples given 2 times (4, 1): give ?samples= once".to_owned())
        );
    }

    #[test]
    fn a_demo_that_takes_no_samples_refuses_them_naming_itself() {
        assert_eq!(none_for("image", None), Ok(()));
        let refused = none_for("image", Some(Samples::Four)).expect_err("refused");
        assert!(refused.starts_with("image draws at 1 sample"), "{refused}");
    }
}
