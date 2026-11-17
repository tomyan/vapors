use vapors_renderer::Samples;

use crate::args::options;

fn args(line: &str) -> Result<crate::args::Options, String> {
    options(line.split_whitespace().map(String::from))
}

#[test]
fn the_full_run_draws_at_1_sample_unless_told_4() {
    assert_eq!(args("").expect("parses").samples, Samples::One);
    assert_eq!(args("--samples 4").expect("parses").samples, Samples::Four);
    assert_eq!(args("--samples 1").expect("parses").samples, Samples::One);
}

#[test]
fn a_sample_count_other_than_1_or_4_is_refused_naming_it() {
    let refused = args("--samples 8").expect_err("refused");
    assert!(
        refused.starts_with("--samples 8: the painter draws at 1 or 4"),
        "{refused}"
    );
}

#[test]
fn samples_with_no_value_are_refused() {
    let refused = args("--samples").expect_err("refused");
    assert!(refused.starts_with("--samples needs 1 or 4"), "{refused}");
}

#[test]
fn samples_with_another_mode_are_refused_naming_it() {
    let refused = args("--samples 4 --mount").expect_err("refused");
    assert!(
        refused.starts_with("--samples and --mount cannot be combined"),
        "{refused}"
    );
}

#[test]
fn samples_go_with_a_scene_run() {
    let given = args("--scene-run one-scene --samples 4").expect("parses");
    assert_eq!(given.samples, Samples::Four);
}
