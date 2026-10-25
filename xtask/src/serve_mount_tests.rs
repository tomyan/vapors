//! Reading what is mounted, and refusing to replace it unasked.

use super::*;

const STATUS: &str = r#"{"Web":{"box.ts.net:443":{"Handlers":{"/":{"Proxy":"http://127.0.0.1:8080"},"/vapors/":{"Path":"/home/me/srv/vapors-demo"}}}}}"#;

fn at(https: u16, path: &str) -> Mount {
    Mount {
        https,
        path: path.into(),
    }
}

#[test]
fn the_handler_at_a_mount_is_read_and_an_empty_mount_is_none() {
    assert_eq!(
        served_at(STATUS, "box.ts.net", &at(443, "/"))
            .expect("json")
            .as_deref(),
        Some("Proxy http://127.0.0.1:8080")
    );
    assert_eq!(
        served_at(STATUS, "box.ts.net", &at(443, "/vapors/"))
            .expect("json")
            .as_deref(),
        Some("Path /home/me/srv/vapors-demo")
    );
    assert_eq!(
        served_at(STATUS, "box.ts.net", &at(8445, "/")).expect("json"),
        None
    );
    assert!(served_at("not json", "box.ts.net", &at(443, "/")).is_err());
}

#[test]
fn another_target_is_replaced_only_when_asked() {
    // Given
    let mount = at(443, "/");
    let wanted = "Path /home/me/srv/touche";

    // Then
    let refused = refuse_other(Some("Proxy http://127.0.0.1:8080"), wanted, &mount, false)
        .expect_err("another");
    assert!(
        refused.contains("already serves Proxy http://127.0.0.1:8080"),
        "{refused}"
    );
    assert_eq!(
        refuse_other(Some("Proxy http://127.0.0.1:8080"), wanted, &mount, true),
        Ok(())
    );
    assert_eq!(
        refuse_other(Some(wanted), wanted, &mount, false),
        Ok(()),
        "the same target again"
    );
    assert_eq!(refuse_other(None, wanted, &mount, false), Ok(()));
}
