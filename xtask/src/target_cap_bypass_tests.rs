//! Every place xtask starts cargo or trunk to build goes through the
//! cache and the build directory's cap (`sccache::apply`); a bare
//! `Command::new("cargo")` is allowed only where it compiles nothing,
//! named with why. A new bare one fails here until it is routed or
//! named.

/// The files with a bare cargo command, and why each compiles nothing.
const BARE: [(&str, &str); 5] = [
    ("builds.rs", "cargo tree"),
    ("layers.rs", "cargo metadata"),
    ("screenshot_package.rs", "cargo metadata"),
    ("web.rs", "cargo pkgid, before trunk, which is capped"),
    (
        "queue_serve.rs",
        "cargo xtask serve, whose own steps are capped",
    ),
];

#[test]
fn every_bare_cargo_command_is_one_that_compiles_nothing() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found: Vec<String> = std::fs::read_dir(&dir)
        .expect("xtask/src")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| !path.to_string_lossy().ends_with("_tests.rs"))
        .filter(|path| {
            let text = std::fs::read_to_string(path).unwrap_or_default();
            text.contains(concat!("Command::new(", "\"cargo\")"))
        })
        .map(|path| {
            path.file_name()
                .expect("a name")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    found.sort();
    let mut allowed: Vec<String> = BARE.iter().map(|(file, _)| (*file).to_owned()).collect();
    allowed.sort();
    assert_eq!(
        found, allowed,
        "a bare cargo command builds without the cache and the cap; fix: start it with \
         crate::priority::command and crate::sccache::apply, or add it to BARE with why it compiles nothing"
    );
}
