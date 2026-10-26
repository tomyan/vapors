use super::*;

const XTASK: [&str; 3] = ["ci", "gate", "web"];

fn examples(package: &str, example: &str) -> bool {
    package == "vapors-app" && example == "demo"
}

fn app() -> App {
    App {
        package: "touche-app".into(),
        commands: vec!["apply".into(), "measure".into()],
    }
}

#[test]
fn named_xtask_commands_in_blocks_and_prose_must_exist() {
    let text = "## Commands\n\n```\ncargo xtask ci          the fast tiers\ncargo xtask gat          a typo\n```\n\nRun `cargo xtask web` then `cargo xtask serv`.\n";
    let problems = check("CLAUDE.md", text, &XTASK, Some(&app()), &examples);
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert!(
        problems[0].starts_with("CLAUDE.md:5: `cargo xtask gat` is not an xtask command"),
        "{problems:?}"
    );
    assert!(
        problems[1].starts_with("CLAUDE.md:8: `cargo xtask serv` is not an xtask command"),
        "{problems:?}"
    );
    assert!(problems[0].contains("fix: "), "{problems:?}");
}

#[test]
fn an_example_must_exist_in_its_package() {
    let text = "```\ncargo run -p vapors-app --example demo -- hello\nxvfb-run -a cargo run -p vapors-app --example demo2\n```\n";
    let problems = check("CLAUDE.md", text, &XTASK, None, &examples);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].starts_with("CLAUDE.md:3: `--example demo2` is not an example of vapors-app"),
        "{problems:?}"
    );
}

#[test]
fn an_app_subcommand_must_be_in_its_help() {
    let text = "```\ncargo run -p touche-app -- apply --curve rgb:0,0 in.png out.png\ncargo run -p touche-app -- render in.png\n```\n";
    let problems = check("CLAUDE.md", text, &XTASK, Some(&app()), &examples);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].starts_with("CLAUDE.md:3: `touche-app render` is not a touche-app command"),
        "{problems:?}"
    );
}

#[test]
fn a_cargo_run_in_no_known_shape_is_refused_naming_the_line() {
    let text = "```\ncargo run -p touche-app\ncargo run -p other-app -- apply\n```\n";
    let problems = check("CLAUDE.md", text, &XTASK, Some(&app()), &examples);
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert!(
        problems[0].starts_with(
            "CLAUDE.md:2: `cargo run -p touche-app` names no example and no subcommand"
        ),
        "{problems:?}"
    );
    assert!(problems[1].starts_with("CLAUDE.md:3: `cargo run -p other-app -- apply` runs other-app, which [docs] app does not name"), "{problems:?}");
}

#[test]
fn the_app_s_help_lists_its_commands() {
    let help = "Usage: touche <COMMAND>\n\nCommands:\n  open     Open an image\n  apply    Render a layer\n  help     Print this message\n\nOptions:\n  -h, --help\n";
    assert_eq!(
        commands_in_help(help),
        Ok(vec![
            "open".to_owned(),
            "apply".to_owned(),
            "help".to_owned()
        ])
    );
    assert!(
        commands_in_help("Usage: x\n")
            .expect_err("no list")
            .contains("Commands:")
    );
}

#[test]
fn a_cargo_run_without_a_package_is_refused_and_every_one_on_a_line_is_read() {
    // Given: prose naming cargo run, one without -p, and two commands on one line
    let text = "every cargo run in the worktree\n`cargo run --example demo`\n`cargo run -p touche-app -- apply` then `cargo run -p touche-app -- aply`\n";

    // When
    let problems = check("CLAUDE.md", text, &XTASK, Some(&app()), &examples);

    // Then
    assert_eq!(problems.len(), 3, "{problems:?}");
    assert!(
        problems[0].starts_with("CLAUDE.md:1: `cargo run in the worktree` names no package (-p)"),
        "{problems:?}"
    );
    assert!(
        problems[1].starts_with("CLAUDE.md:2: `cargo run --example demo` names no package (-p)"),
        "{problems:?}"
    );
    assert!(
        problems[2].starts_with("CLAUDE.md:3: `touche-app aply` is not a touche-app command"),
        "{problems:?}"
    );
}

#[test]
fn a_wrapped_command_description_is_not_read_as_a_command() {
    let help = "Commands:\n  open     Open an image\n           that is big\n  apply    Render a layer\n\nOptions:\n";
    assert_eq!(
        commands_in_help(help),
        Ok(vec!["open".to_owned(), "apply".to_owned()])
    );
}
