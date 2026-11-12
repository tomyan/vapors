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
