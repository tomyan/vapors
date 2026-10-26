//! The `.events` input scripts: one directive a line, `#` comments, a
//! note after the fields.

use super::*;

const SCRIPT: &str = "\
# The card's Edit button by mouse, then the field by keyboard.
at 412x915
listen click focusin keydown input pointerdown
click 120 300                 the Edit button
expect event click button#edit
expect focus button#edit
key Shift+Tab
expect focus-visible input#title
type \"Warm\"                  four keys
expect event input input#title
expect no event click
wheel 200 400 0 120
expect event wheel div#list
tap 50 60
expect focus button#edit
frame
";

#[test]
fn a_script_reads_as_its_viewport_its_listened_types_and_its_steps() {
    // When
    let script = parse("card", SCRIPT).expect("parses");

    // Then
    assert_eq!(script.viewport, Some((412, 915)));
    assert_eq!(
        script.listen,
        ["click", "focusin", "keydown", "input", "pointerdown"]
    );
    let actions: Vec<&Action> = script.steps.iter().map(|s| &s.action).collect();
    assert_eq!(
        actions,
        [
            &Action::Click(120.0, 300.0),
            &Action::Key("Shift+Tab".into()),
            &Action::Type("Warm".into()),
            &Action::Wheel {
                x: 200.0,
                y: 400.0,
                dx: 0.0,
                dy: 120.0
            },
            &Action::Tap(50.0, 60.0),
            &Action::Frame,
        ]
    );
    assert_eq!(
        script.steps[0].expect,
        [
            (
                Expect::Event {
                    kind: "click".into(),
                    target: "button#edit".into()
                },
                5
            ),
            (Expect::Focus("button#edit".into()), 6),
        ]
    );
    assert_eq!(
        script.steps[2].expect[1],
        (Expect::NoEvent("click".into()), 11)
    );
    assert_eq!(script.steps[2].line, 9);
}

#[test]
fn a_script_of_another_shape_is_refused_naming_its_file_and_line() {
    for (text, needle) in [
        (
            "listen click\nclik 1 2\n",
            "card.events:2: no directive `clik`",
        ),
        (
            "listen click\nexpect focus a#b\n",
            "card.events:2: an expectation before any action",
        ),
        (
            "listen click\nclick 1 2\nexpect no event input\n",
            "card.events:3: `input` is not listened for",
        ),
        ("click 1 2\n", "card.events:1: an action before `listen`"),
        (
            "listen click\nlisten input\n",
            "card.events:2: a second `listen`",
        ),
        ("at 412x915\nat 412x915\n", "card.events:2: a second `at`"),
        (
            "listen click\ntype Warm\n",
            "card.events:2: `type` takes a quoted text",
        ),
        (
            "listen click\nclick 1\n",
            "card.events:2: `click` takes X Y",
        ),
        (
            "listen click\nkey Hyper+a\n",
            "card.events:2: no modifier `Hyper`",
        ),
        (
            "listen click\nclick 1 2\nexpect no event click button#save\n",
            "card.events:3: extra fields",
        ),
        (
            "listen click\nclick 1 2\nexpect event click a#b c#d\n",
            "card.events:3: extra fields",
        ),
        (
            "listen click\nmove 10 20 30\n",
            "card.events:2: extra fields",
        ),
        (
            "listen click\nclick NaN 5\n",
            "card.events:2: `click` takes X Y",
        ),
        (
            "listen click\nclick inf 5\n",
            "card.events:2: `click` takes X Y",
        ),
        ("listen click\nkey Tabb\n", "card.events:2: no key `Tabb`"),
        ("listen click\nkey tab\n", "card.events:2: no key `tab`"),
        ("listen click\nkey Shift+\n", "card.events:2: no key ``"),
        (
            "listen click\ntype \"\"\n",
            "card.events:2: `type` takes a quoted text",
        ),
        ("at 0x0\n", "card.events:1: `at` takes WxH"),
    ] {
        let refusal = parse("card", text).expect_err(needle);
        assert!(refusal.starts_with(needle), "wanted {needle} in: {refusal}");
    }
}

#[test]
fn a_script_with_nothing_to_check_is_refused() {
    for (text, needle) in [
        ("", "card.events: no steps"),
        ("listen click\n", "card.events: no steps"),
        (
            "listen click\nclick 1 2\n",
            "card.events:2: a step with no expectation",
        ),
    ] {
        let refusal = parse("card", text).expect_err(needle);
        assert!(refusal.starts_with(needle), "wanted {needle} in: {refusal}");
    }
}

#[test]
fn a_note_follows_two_spaces_and_is_not_a_field() {
    let script = parse(
        "card",
        "listen click\nclick 1 2  the Save button, at its centre\nexpect focus a#b  focused\n",
    )
    .expect("parses");
    assert_eq!(script.steps[0].action, Action::Click(1.0, 2.0));
    assert_eq!(script.steps[0].expect[0].0, Expect::Focus("a#b".into()));
}
