use super::*;

#[test]
fn the_listening_process_and_pid_are_read_from_ss() {
    let ss = "LISTEN 0      1024       127.0.0.1:8790      0.0.0.0:*    users:((\"static-web-serv\",pid=41235,fd=9))\n";
    assert_eq!(
        listener(ss),
        Some(("static-web-serv".to_owned(), "41235".to_owned()))
    );
}

#[test]
fn a_line_without_a_process_reads_as_none() {
    assert_eq!(listener("LISTEN 0 1024 127.0.0.1:8790 0.0.0.0:*\n"), None);
    assert_eq!(listener(""), None);
}

#[test]
fn a_port_held_by_another_process_is_refused_naming_it() {
    // Given: this test's own listener, which no unit owns
    let held = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = held.local_addr().expect("addr").port();

    // When
    let refusal = owned_by("xtask-serve-no-such-unit.service", port).expect_err("refused");

    // Then
    assert!(
        refusal.starts_with(&format!("127.0.0.1:{port} is held by ")),
        "{refusal}"
    );
    assert!(
        refusal.contains(&format!("pid {}", std::process::id())),
        "{refusal}"
    );
    assert!(refusal.contains("fix: change [serve] port"), "{refusal}");
}
