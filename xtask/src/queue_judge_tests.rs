use super::*;

#[test]
fn a_change_to_the_factory_ci_or_build_settings_is_protected() {
    for path in [
        "xtask/src/queue.rs",
        "xtask/systemd/queue@.timer",
        ".github/workflows/ci.yml",
        ".config/factory.toml",
        ".config/vapors-rev",
        ".cargo/config.toml",
        "rust-toolchain.toml",
        "deny.toml",
    ] {
        assert_eq!(
            protected(&[path.to_owned()]),
            Some(path.to_owned()),
            "{path}"
        );
    }
}

#[test]
fn other_changes_are_not_protected() {
    let touched = [
        "crates/web/page.js",
        "docs/PLAN.md",
        "Cargo.lock",
        "README.md",
    ]
    .map(String::from);
    assert_eq!(protected(&touched), None);
}
