use super::*;

const A: &str = "a71fb43143d0964f82417178fa56d8f6ee4b33f49ddde4b2263d97404fdfbbc4";
const B: &str = "4b39efe44e995bfb1a19517079046490ccc9b4e9907bf924ec46bef3197be56b";

fn file(extra: &str) -> String {
    format!(
        "[tools]\nbacon = \"3\"\n\n[downloads]\nsws.version = \"2.44.1\"\nsws.url = \"https://example.org/sws.tar.gz\"\nsws.sha256 = \"{A}\"\nsws.member = \"sws/sws\"\nsws.binary_sha256 = \"{B}\"\n{extra}"
    )
}

#[test]
fn a_download_is_read_with_every_field() {
    let downloads = parse(&file("")).expect("valid");
    assert_eq!(
        downloads,
        vec![Download {
            archive: crate::downloads::Archive::TarGz,
            tree: None,
            name: "sws".into(),
            version: "2.44.1".into(),
            url: "https://example.org/sws.tar.gz".into(),
            sha256: A.into(),
            member: "sws/sws".into(),
            binary_sha256: B.into(),
        }]
    );
}

#[test]
fn it_is_installed_under_the_home_directory_by_name_and_version() {
    let download = parse(&file("")).expect("valid").remove(0);
    assert_eq!(
        installed_path(&download, Path::new("/home/t")),
        Path::new("/home/t/.local/share/factory/bin/sws-2.44.1")
    );
}

#[test]
fn a_malformed_field_is_refused_naming_its_line() {
    let cases = [
        ("sws.sha256 = \"abc\"", "64 lower-case hex"),
        ("sws.url = \"http://example.org/sws.tar.gz\"", "https URL"),
        ("sws.colour = \"red\"", "unknown field"),
        ("sws.version = \"2.44.2\"", "given twice"),
    ];
    for (line, said) in cases {
        let error = parse(&file(line)).expect_err(line);
        assert!(error.reason.contains(said), "{line}: {error}");
        assert_eq!(error.line, 10, "{line}");
    }
}

#[test]
fn a_missing_field_is_refused_naming_it() {
    let text = file("").replace(&format!("sws.binary_sha256 = \"{B}\"\n"), "");
    let error = parse(&text).expect_err("missing");
    assert!(error.reason.contains("sws has no binary_sha256"), "{error}");
}

const TREE: &str = "cr.version = \"157\"\ncr.url = \"https://example.org/o/cr.zip?alt=media\"\ncr.archive = \"zip\"\ncr.tree = \"chrome-linux\"\ncr.member = \"chrome-linux/chrome\"\n";

fn tree_file(extra: &str) -> String {
    file(&format!(
        "{TREE}cr.sha256 = \"{A}\"\ncr.binary_sha256 = \"{B}\"\n{extra}"
    ))
}

#[test]
fn a_zip_tree_is_installed_under_its_own_directory() {
    // Given
    let downloads = parse(&tree_file("")).expect("valid");

    // When
    let chromium = downloads.iter().find(|d| d.name == "cr").expect("cr");

    // Then
    assert_eq!(
        (chromium.archive, chromium.tree.as_deref()),
        (Archive::Zip, Some("chrome-linux"))
    );
    assert_eq!(
        installed_path(chromium, Path::new("/home/t")),
        Path::new("/home/t/.local/share/factory/cr-157/chrome-linux/chrome")
    );
}

#[test]
fn a_tree_whose_member_is_outside_it_is_refused() {
    let text = tree_file("").replace(
        "cr.member = \"chrome-linux/chrome\"",
        "cr.member = \"chrome\"",
    );
    let refused = parse(&text).expect_err("member outside the tree");
    assert!(refused.to_string().contains("inside tree"), "{refused}");
}

#[test]
fn a_zip_archive_whose_url_names_no_zip_is_refused() {
    let text = tree_file("").replace("cr.zip?alt=media", "cr.tar.gz");
    let refused = parse(&text).expect_err("not a zip");
    assert!(refused.to_string().contains(".zip"), "{refused}");
}
