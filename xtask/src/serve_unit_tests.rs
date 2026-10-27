use super::*;

fn paths(dir: &'static str) -> Paths<'static> {
    Paths {
        server: Path::new("/home/t/.local/share/factory/bin/static-web-server-2.44.1"),
        dir: Path::new(dir),
        copy: Path::new("/home/t/srv/touche.3"),
        config: Path::new("/home/t/srv/touche.sws.toml"),
    }
}

#[test]
fn hashed_files_are_immutable_after_everything_is_no_cache_and_listings_are_off() {
    // When
    let config = config_text();

    // Then: the later entry wins in static-web-server, so it comes second
    let all = config.find("source = \"**/*\"\n").expect("all files");
    let hashed = format!(
        "source = \"**/*-{}{{_bg.wasm,.js,.css}}\"\n",
        "[0-9a-f]".repeat(16)
    );
    assert!(
        all < config.find(&hashed).expect("hashed files"),
        "{config}"
    );
    assert!(config.contains("compression-static = true\n"), "{config}");
    assert!(config.contains("directory-listing = false\n"), "{config}");
    assert!(
        config.contains("Cache-Control = \"public, max-age=31536000, immutable\""),
        "{config}"
    );
}

#[test]
fn the_unit_runs_the_verified_binary_on_the_loopback_only_quoted() {
    let unit = unit_text(&paths("/home/t/srv/touche"), 8790).expect("unit");
    assert!(
        unit.contains("ExecStart=\"/home/t/.local/share/factory/bin/static-web-server-2.44.1\" --host 127.0.0.1 --port 8790 --root \"/home/t/srv/touche\" --config-file \"/home/t/srv/touche.sws.toml\"\n"),
        "{unit}"
    );
}

#[test]
fn the_unit_is_sandboxed_with_only_the_copy_config_and_binary_readable() {
    let unit = unit_text(&paths("/home/t/srv/touche"), 8790).expect("unit");
    for line in [
        "NoNewPrivileges=yes",
        "ProtectSystem=strict",
        "ProtectHome=tmpfs",
        "PrivateTmp=yes",
    ] {
        assert!(unit.contains(&format!("{line}\n")), "{line}: {unit}");
    }
    assert!(
        unit.contains("BindReadOnlyPaths=/home/t/srv/touche.3:/home/t/srv/touche /home/t/srv/touche.sws.toml /home/t/.local/share/factory/bin/static-web-server-2.44.1\n"),
        "{unit}"
    );
    assert!(!unit.contains("BindPaths="), "{unit}");
}

#[test]
fn a_path_the_unit_cannot_carry_is_refused_naming_it() {
    for dir in [
        "/home/t/my srv/touche",
        "/home/t/srv/50%",
        "/home/t/srv/a:b",
        "/home/t/srv/q\"",
    ] {
        let refusal = unit_text(&paths(dir), 8790).expect_err(dir);
        assert!(
            refusal.starts_with(dir) && refusal.contains("fix: "),
            "{refusal}"
        );
    }
}

#[test]
fn the_unit_is_named_for_the_served_directory() {
    assert_eq!(
        unit_name(Path::new("/home/t/srv/touche")),
        "xtask-serve-touche.service"
    );
}
