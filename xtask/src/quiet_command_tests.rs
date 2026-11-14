//! `cargo xtask quiet`'s forms.

use std::time::Duration;

use super::*;
use crate::quiet_wait::HOLDER_ENV;

fn parsed(args: &[&str]) -> Result<Action, String> {
    let args: Vec<String> = args.iter().map(|&a| a.to_owned()).collect();
    Action::parse(&args)
}

#[test]
fn minutes_and_a_command_run_the_command_in_quiet() {
    assert_eq!(
        parsed(&["10", "--", "cargo", "xtask", "perf"]),
        Ok(Action::Run(
            Duration::from_mins(10),
            vec!["cargo".into(), "xtask".into(), "perf".into()]
        ))
    );
}

#[test]
fn hold_takes_fifteen_minutes_unless_told_otherwise() {
    assert_eq!(parsed(&["hold"]), Ok(Action::Hold(Duration::from_mins(15))));
    assert_eq!(
        parsed(&["hold", "5"]),
        Ok(Action::Hold(Duration::from_mins(5)))
    );
    assert_eq!(parsed(&["release"]), Ok(Action::Release));
}

#[test]
fn any_other_form_is_refused_naming_the_three_forms() {
    for args in [
        &[][..],
        &["10"][..],
        &["10", "--"][..],
        &["10", "true"][..],
        &["0", "--", "true"][..],
        &["soon", "--", "true"][..],
        &["hold", "x"][..],
        &["hold", "5", "6"][..],
        &["hold", "61"][..],
        &["400000000000000000", "--", "true"][..],
        &["release", "now"][..],
    ] {
        let refusal = parsed(args).expect_err(&format!("{args:?}"));
        assert!(
            refusal.contains("cargo xtask quiet hold"),
            "{args:?}: {refusal}"
        );
    }
}

#[test]
fn the_holders_own_commands_are_told_who_holds_it() {
    // When
    let command = child(&["true".to_owned()]);

    // Then
    let told = command.get_envs().any(|(name, value)| {
        name == HOLDER_ENV && value == Some(std::process::id().to_string().as_ref())
    });
    assert!(told);
}
