//! `expect` directives: a control's value, a checkbox's state and a
//! text field's selection, each named and refused when malformed.

use crate::events_script::{Expect, parse};

#[test]
fn a_value_expectation_names_the_control_and_its_value() {
    // When
    let script = parse(
        "r",
        "listen input\nkey ArrowRight\nexpect value input#b 51\nexpect value input#a 20\nkey End\nexpect value input#b 100\n",
    )
    .expect("parses");

    // Then
    assert_eq!(
        script.steps[0].expect[0],
        (
            Expect::Value {
                target: "input#b".into(),
                value: "51".into()
            },
            3
        )
    );
    assert_eq!(script.value_targets(), ["input#a", "input#b"]);
    let empty =
        parse("r", "listen input\nkey Delete\nexpect value input#a \"\"\n").expect("parses");
    assert_eq!(
        empty.steps[0].expect[0],
        (
            Expect::Value {
                target: "input#a".into(),
                value: String::new()
            },
            3
        )
    );
}

#[test]
fn a_checked_expectation_takes_true_or_false() {
    // When
    let script = parse(
        "c",
        "listen click\nclick 5 5\nexpect checked input#b true\nexpect checked input#a false\n",
    )
    .expect("parses");
    let refused =
        parse("c", "listen click\nclick 5 5\nexpect checked input#b yes\n").expect_err("refused");

    // Then
    assert_eq!(
        script.steps[0].expect[0],
        (
            Expect::Checked {
                target: "input#b".into(),
                checked: true
            },
            3
        )
    );
    assert_eq!(script.checked_targets(), ["input#a", "input#b"]);
    assert!(
        refused.contains("c.events:3:") && refused.contains("true or false"),
        "{refused}"
    );
}

#[test]
fn a_selection_expectation_takes_a_start_at_most_its_end() {
    // When
    let script = parse(
        "t",
        "listen input\nkey End\nexpect selection input#b 4 4\nexpect selection input#a 0 2\n",
    )
    .expect("parses");
    let backwards =
        parse("t", "listen input\nkey End\nexpect selection input#a 3 1\n").expect_err("refused");
    let words = parse(
        "t",
        "listen input\nkey End\nexpect selection input#a end 1\n",
    )
    .expect_err("refused");

    // Then
    assert_eq!(
        script.steps[0].expect[0],
        (
            Expect::Selection {
                target: "input#b".into(),
                start: 4,
                end: 4
            },
            3
        )
    );
    assert_eq!(script.selection_targets(), ["input#a", "input#b"]);
    for refused in [backwards, words] {
        assert!(
            refused.contains("t.events:3:") && refused.contains("start at most end"),
            "{refused}"
        );
    }
}
