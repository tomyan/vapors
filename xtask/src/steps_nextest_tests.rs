//! Nothing the factory or CI runs overrides .config/nextest.toml's
//! timeouts: the text comparison's launcher deadline is checked against
//! that file (vapors-text `chromium_process` tests), so an override on
//! the command line or in the environment would void the check.
//! `NEXTEST_PROFILE` only picks a profile, and the check reads them all.

use super::*;

const OVERRIDING_FLAGS: [&str; 3] = ["--slow-timeout", "--config-file", "--tool-config-file"];

#[test]
fn no_step_passes_a_flag_that_overrides_nextest_toml() {
    for step in ci("vapors-web-demo") {
        for arg in &step.args {
            assert!(
                !OVERRIDING_FLAGS.iter().any(|flag| arg.starts_with(flag)),
                "{}: `{arg}` overrides .config/nextest.toml",
                step.name
            );
        }
    }
}

#[test]
fn ci_yml_sets_no_nextest_variable_but_the_profile_and_no_overriding_flag() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../.github/workflows/ci.yml");
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    for (index, line) in text.lines().enumerate() {
        let key = line.trim().split(':').next().unwrap_or_default();
        let at = format!("{path}:{}", index + 1);
        assert!(
            !key.starts_with("NEXTEST_") || key == "NEXTEST_PROFILE",
            "{at}: sets {key}"
        );
        for flag in OVERRIDING_FLAGS {
            assert!(
                !line.contains(flag),
                "{at}: `{flag}` overrides .config/nextest.toml"
            );
        }
    }
}
