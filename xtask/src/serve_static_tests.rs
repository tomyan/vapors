use super::*;

#[test]
fn hashed_files_are_immutable_after_everything_is_no_cache() {
    // When
    let config = config_text();

    // Then: the later entry wins in static-web-server, so it comes second
    let all = config.find("source = \"**/*\"").expect("all files");
    let hashed = config
        .find("source = \"**/*-[0-9a-f]")
        .expect("hashed files");
    assert!(all < hashed, "{config}");
    assert!(
        config.contains(&format!(
            "source = \"**/*-{}{{_bg.wasm,.js}}\"\n",
            "[0-9a-f]".repeat(16)
        )),
        "{config}"
    );
    assert!(config.contains("compression-static = true"), "{config}");
    assert!(
        config.contains("Cache-Control = \"public, max-age=31536000, immutable\""),
        "{config}"
    );
}

#[test]
fn the_unit_binds_the_loopback_only_and_resolves_the_root_per_request() {
    // When
    let unit = unit_text(
        Path::new("/bin/sws"),
        Path::new("/home/t/srv/touche"),
        8790,
        Path::new("/home/t/srv/touche.sws.toml"),
    );

    // Then
    assert!(unit.contains("--host 127.0.0.1 --port 8790"), "{unit}");
    assert!(
        unit.contains("--root /home/t/srv/touche --use-relative-root"),
        "{unit}"
    );
    assert!(
        unit.contains("--config-file /home/t/srv/touche.sws.toml"),
        "{unit}"
    );
}

#[test]
fn the_unit_is_named_for_the_served_directory() {
    assert_eq!(
        unit_name(Path::new("/home/t/srv/touche")),
        "xtask-serve-touche.service"
    );
}
