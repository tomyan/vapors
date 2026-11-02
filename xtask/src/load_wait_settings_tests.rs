//! `[load]`'s keys: defaults, settings, and refusals naming the key.

use super::*;

const DEFAULTS: Settings = Settings {
    enabled: true,
    threshold: 12.0,
    deadline: DEFAULT_DEADLINE,
};

fn parsed(text: &str) -> Result<Settings, String> {
    settings(&Config::parse(text).expect(text), 12)
}

#[test]
fn the_threshold_is_the_core_count_unless_the_config_sets_it() {
    // When
    let (default, configured) = (
        parsed(""),
        parsed("[load]\nthreshold = \"8\"\ndeadline_minutes = \"2\"\n"),
    );

    // Then
    assert_eq!(default, Ok(DEFAULTS));
    assert_eq!(
        configured,
        Ok(Settings {
            threshold: 8.0,
            deadline: Duration::from_mins(2),
            ..DEFAULTS
        })
    );
}

#[test]
fn wait_false_turns_the_wait_off() {
    assert_eq!(
        parsed("[load]\nwait = \"false\"\n"),
        Ok(Settings {
            enabled: false,
            ..DEFAULTS
        })
    );
}

#[test]
fn a_setting_of_the_wrong_shape_is_refused_naming_it() {
    for text in [
        "[load]\nthreshold = \"0\"\n",
        "[load]\ndeadline_minutes = \"soon\"\n",
        "[load]\nwait = \"no\"\n",
        "[load]\ndeadline_minutes = \"1e300\"\n",
    ] {
        // When
        let refusal = parsed(text).expect_err(text);

        // Then
        assert!(refusal.starts_with("[load] "), "{refusal}");
        assert!(refusal.contains("fix: .config/factory.toml"), "{refusal}");
    }
}
