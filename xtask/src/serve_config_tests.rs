//! `[serve]` and `[dev] https` read, and refused when malformed.

use super::*;

fn config(text: &str) -> Config {
    Config::parse(text).expect("parses")
}

const SERVE: &str = "[serve]\ndir = \"srv/touche\"\nhttps = \"443\"\npath = \"/\"\ncheck = \"true\"\ncheck_seconds = \"60\"\nport = \"8790\"\n";

#[test]
fn serve_is_read_with_its_dir_under_home() {
    // When
    let serve = serve(&config(SERVE), Path::new("/home/me")).expect("read");

    // Then
    assert_eq!(serve.dir, Path::new("/home/me/srv/touche"));
    assert_eq!(
        serve.mount,
        Mount {
            https: 443,
            path: "/".into()
        }
    );
    assert_eq!(serve.check, "true");
}

#[test]
fn a_missing_or_malformed_serve_key_is_refused_by_name() {
    for (text, named) in [
        (SERVE.replace("check = \"true\"\n", ""), "serve.check"),
        (SERVE.replace("\"443\"", "\"web\""), "serve.https"),
        (
            SERVE.replace("path = \"/\"", "path = \"vapors\""),
            "serve.path",
        ),
        (
            SERVE.replace("path = \"/\"", "path = \"/vapors\""),
            "serve.path",
        ),
        (SERVE.replace("srv/touche", "/srv/touche"), "serve.dir"),
        (SERVE.replace("srv/touche", "."), "serve.dir"),
        (SERVE.replace("srv/touche", "srv/../touche"), "serve.dir"),
        (SERVE.replace("srv/touche", "srv//touche"), "serve.dir"),
        (SERVE.replace("\"60\"", "\"0\""), "serve.check_seconds"),
    ] {
        let refused = serve(&config(&text), Path::new("/home/me")).expect_err(named);
        assert!(refused.contains(named), "{named}: {refused}");
    }
}

#[test]
fn the_dev_page_is_at_the_root_of_its_own_port() {
    // When
    let mount = dev(&config("[dev]\nhttps = \"8445\"\n")).expect("read");

    // Then
    assert_eq!(
        mount,
        Mount {
            https: 8445,
            path: "/".into()
        }
    );
    assert!(dev(&config("[dev]\n")).is_err(), "no port: refused");
}
