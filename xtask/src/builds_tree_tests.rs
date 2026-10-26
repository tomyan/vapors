use super::*;

const TREE: &str = "\
0touche-app v0.1.0 (/w/crates/app)
1touche-runtime v0.1.0 (/w/crates/runtime)
2reqwest v0.12.0
3hyper v1.0.0
1serde v1.0.210
2hyper v1.0.0 (*)
";

fn server_only(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| (*n).to_owned()).collect()
}

#[test]
fn a_server_only_crate_is_named_with_the_path_that_pulled_it_in() {
    assert_eq!(
        pulled_in(TREE, &server_only(&["hyper", "tokio"])),
        Ok(vec![
            "hyper, pulled in by touche-app -> touche-runtime -> reqwest -> hyper".to_owned()
        ])
    );
}

#[test]
fn a_tree_without_a_server_only_crate_passes() {
    assert_eq!(pulled_in(TREE, &server_only(&["tokio"])), Ok(vec![]));
    assert_eq!(pulled_in(TREE, &[]), Ok(vec![]));
}

#[test]
fn a_crate_whose_name_only_starts_with_a_listed_one_is_not_it() {
    assert_eq!(pulled_in(TREE, &server_only(&["touche"])), Ok(vec![]));
}

#[test]
fn a_tree_line_of_another_shape_is_refused_naming_it() {
    let tree = "0touche-app v0.1.0 (/w)\n[build-dependencies]\n";
    assert_eq!(
        pulled_in(tree, &[]),
        Err(
            "cargo tree's line 2 is not `<depth><name> v<version>`: \"[build-dependencies]\""
                .to_owned()
        )
    );
}

#[test]
fn a_depth_that_skips_a_level_is_refused() {
    let tree = "0touche-app v0.1.0 (/w)\n2hyper v1.0.0\n";
    assert_eq!(
        pulled_in(tree, &[]),
        Err(
            "cargo tree's line 2 is deeper than the line before it allows: \"2hyper v1.0.0\""
                .to_owned()
        )
    );
}
