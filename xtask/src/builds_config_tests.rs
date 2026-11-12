use super::*;
use crate::config::Config;

fn separation(text: &str) -> Result<Separation, String> {
    Separation::from(&Config::parse(text).expect("valid"))
}

const BUILDS: &str = "[server_only.builds]\ndesktop = \"touche-app host\"\nweb = \"touche-web wasm32-unknown-unknown\"\n";

#[test]
fn the_server_only_crates_and_each_shipped_build_are_read() {
    // When
    let read = separation(&format!(
        "[server_only]\ncrates = \"hub-server  billing\"\n\n{BUILDS}"
    ));

    // Then
    let read = read.expect("read");
    assert_eq!(read.crates, ["hub-server", "billing"]);
    assert_eq!(
        read.builds,
        [
            Build {
                name: "desktop".into(),
                package: "touche-app".into(),
                target: None
            },
            Build {
                name: "web".into(),
                package: "touche-web".into(),
                target: Some("wasm32-unknown-unknown".into())
            },
        ]
    );
}

#[test]
fn no_crates_needs_a_reason_and_a_list_refuses_one() {
    let reason = "none_because = \"nothing SaaS- or corp-only exists yet\"\n";
    let none = separation(&format!("[server_only]\n{reason}\n{BUILDS}"))
        .expect("no crates, with a reason");
    assert_eq!(none.crates, Vec::<String>::new());
    assert_eq!(
        separation(BUILDS).expect_err("no reason"),
        "[server_only] lists no crates and none_because gives no reason; fix: say why nothing is server-only yet, or list the crates"
    );
    assert_eq!(
        separation(&format!(
            "[server_only]\ncrates = \"billing\"\n{reason}\n{BUILDS}"
        ))
        .expect_err("stale reason"),
        "[server_only] lists crates, so none_because no longer holds; fix: remove none_because"
    );
}

#[test]
fn missing_settings_or_a_build_of_another_shape_are_refused_naming_them() {
    let no_builds = separation("[server_only]\ncrates = \"billing\"\n").expect_err("no builds");
    assert!(
        no_builds.starts_with("[server_only.builds] names no shipped build"),
        "{no_builds}"
    );
    let odd = separation(
        "[server_only]\ncrates = \"billing\"\n\n[server_only.builds]\ndesktop = \"touche-app\"\n",
    )
    .expect_err("shape");
    assert_eq!(
        odd,
        "[server_only.builds] desktop is `touche-app`, not `<package> <target>`; fix: write the package and its target triple, or host"
    );
}
