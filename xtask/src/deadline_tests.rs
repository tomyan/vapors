//! Running a child by a deadline: its exit, its kill, its output.

use super::*;

#[test]
fn a_quick_command_exits_with_its_output() {
    // Given
    let mut command = Command::new("sh");
    command.args(["-c", "echo hello"]);

    // When
    let finished = run_with_deadline(command, Duration::from_secs(5)).expect("spawns");

    // Then
    match finished {
        Finished::Exited { success, output } => {
            assert!(success);
            assert_eq!(output.trim(), "hello");
        }
        Finished::TimedOut { .. } => panic!("should not time out"),
    }
}

#[test]
fn a_slow_command_is_killed_at_the_deadline() {
    // Given
    let mut command = Command::new("sh");
    command.args(["-c", "sleep 10"]);

    // When
    let started = Instant::now();
    let finished = run_with_deadline(command, Duration::from_millis(200)).expect("spawns");

    // Then
    assert!(matches!(finished, Finished::TimedOut { .. }));
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn a_command_killed_at_the_deadline_keeps_its_output_so_far() {
    // Given: the shell's `sleep` child keeps the pipe open past the kill.
    let mut command = Command::new("sh");
    command.args(["-c", "echo partial; sleep 10"]);

    // When
    let started = Instant::now();
    let finished = run_with_deadline(command, Duration::from_millis(500)).expect("spawns");

    // Then
    let Finished::TimedOut { output } = finished else {
        panic!("should time out");
    };
    assert_eq!(output.trim(), "partial");
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn only_the_last_64_kib_of_a_long_output_is_kept() {
    // Given: 200 000 bytes, then a last line
    let mut command = Command::new("sh");
    command.args([
        "-c",
        "head -c 200000 /dev/zero | tr '\\0' a; echo; echo END",
    ]);

    // When
    let finished = run_with_deadline(command, Duration::from_secs(5)).expect("spawns");

    // Then
    let Finished::Exited { output, .. } = finished else {
        panic!("should exit");
    };
    assert!(output.starts_with(DROPPED), "the drop is said");
    assert!(
        output.len() <= DROPPED.len() + KEPT_BYTES,
        "{} bytes kept",
        output.len()
    );
    assert!(output.trim_end().ends_with("END"), "the end is kept");
}

#[test]
fn multibyte_characters_split_across_reads_are_decoded_whole() {
    // Given: one 15 000-byte write of 3-byte characters, so the
    // reader's 4096-byte reads, which 3 does not divide, split some
    let mut command = Command::new("sh");
    command.args(["-c", "printf '€%.0s' $(seq 5000)"]);

    // When
    let finished = run_with_deadline(command, Duration::from_secs(5)).expect("spawns");

    // Then
    let Finished::Exited { output, .. } = finished else {
        panic!("should exit");
    };
    assert_eq!(output.chars().filter(|c| *c == '€').count(), 5000);
    assert!(!output.contains('\u{FFFD}'), "no character was split");
}

#[test]
fn an_exit_whose_pipe_a_background_child_holds_returns_by_the_join_deadline() {
    // Given: the shell exits at once; its background `sleep` keeps stdout open
    let mut command = Command::new("sh");
    command.args(["-c", "echo done; sleep 10 &"]);

    // When
    let started = Instant::now();
    let finished = run_with_deadline(command, Duration::from_secs(5)).expect("spawns");

    // Then
    let Finished::Exited { success, output } = finished else {
        panic!("should exit");
    };
    assert!(success);
    assert!(output.starts_with("done"), "{output}");
    assert!(output.contains("still held open"), "{output}");
    assert!(started.elapsed() < JOIN_DEADLINE + Duration::from_secs(2));
}

#[test]
fn a_drop_that_cuts_a_character_in_half_leaves_it_out() {
    // Given: 90 000 bytes of 3-byte characters; the drop of 24 464 bytes
    // ends two bytes into one
    let mut ring = Ring::default();
    ring.push("€".repeat(30_000).as_bytes());

    // When
    let text = ring.text();

    // Then
    assert!(!text.contains('\u{FFFD}'), "no half character");
    assert_eq!(text.strip_prefix(DROPPED).expect("said").len() % 3, 0);
}

#[test]
fn the_notes_state_the_numbers_the_code_uses() {
    assert!(DROPPED.contains(&format!("{} KiB", KEPT_BYTES / 1024)));
    assert!(STILL_OPEN.contains(&format!("{} s", JOIN_DEADLINE.as_secs())));
}
