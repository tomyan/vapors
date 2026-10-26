use super::*;

#[test]
fn a_host_build_s_tree_names_no_target_and_a_wasm_one_does() {
    let host = Build {
        name: "desktop".into(),
        package: "touche-app".into(),
        target: None,
    };
    let web = Build {
        name: "web".into(),
        package: "touche-web".into(),
        target: Some("wasm32-unknown-unknown".into()),
    };
    assert_eq!(
        tree_command(&host).join(" "),
        "tree -e normal -p touche-app --prefix depth --format {p}"
    );
    assert_eq!(
        tree_command(&web).join(" "),
        "tree -e normal -p touche-web --target wasm32-unknown-unknown --prefix depth --format {p}"
    );
}
