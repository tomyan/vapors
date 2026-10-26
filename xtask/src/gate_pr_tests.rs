use super::*;

#[test]
fn the_tail_is_the_last_lines_of_the_output() {
    // Given
    let lines: Vec<String> = (1..=40).map(|n| format!("line {n}")).collect();
    let output = lines.join("\n") + "\n";

    // When
    let tail = tail(&output, 30);

    // Then
    assert_eq!(tail.lines().count(), 30);
    assert!(tail.starts_with("line 11\n"), "{tail}");
    assert!(tail.ends_with("line 40"), "{tail}");
}

#[test]
fn a_short_output_is_its_own_tail() {
    assert_eq!(tail("a\nb\n", 30), "a\nb");
}

#[test]
fn the_comment_names_the_head_the_command_and_the_exit_code_above_the_tail() {
    // When
    let body = comment("0123abc", 71, 0, "ok   ci\ngate: pass 0123abc");

    // Then
    assert_eq!(
        body,
        "Gate output for 0123abc (`cargo xtask gate --pr 71`, exit 0):\n\n```\nok   ci\ngate: pass 0123abc\n```\n"
    );
}

#[test]
fn a_pr_is_read_from_gh_with_its_number_and_head() {
    // When
    let pr = parse_pr(r#"{"number":71,"headRefOid":"b26d5aa3cf0d40373d621158d8231e3555598eeb"}"#);

    // Then
    assert_eq!(
        pr,
        Ok(Pr {
            number: 71,
            head: "b26d5aa3cf0d40373d621158d8231e3555598eeb".into()
        })
    );
}

#[test]
fn gh_output_without_a_number_or_head_is_refused_naming_it() {
    for (json, missing) in [
        (r#"{"headRefOid":"abc"}"#, "number"),
        (r#"{"number":3}"#, "headRefOid"),
        ("not json", "JSON"),
    ] {
        let refused = parse_pr(json).expect_err(json);
        assert!(refused.contains(missing), "{json}: {refused}");
    }
}

#[test]
fn a_pr_whose_head_is_not_local_head_is_refused_before_the_gate_runs() {
    // Given
    let pr = Pr {
        number: 71,
        head: "aaa".into(),
    };

    // When
    let refused = same_head(&pr, "bbb").expect_err("a mismatch");

    // Then
    assert!(
        refused.contains("#71's head is aaa") && refused.contains("HEAD is bbb"),
        "{refused}"
    );
    assert!(refused.contains("fix: "), "{refused}");
    assert_eq!(same_head(&pr, "aaa"), Ok(()));
}

#[test]
fn a_head_that_moved_during_the_gate_is_not_posted() {
    // Given
    let gated = Pr {
        number: 71,
        head: "aaa".into(),
    };

    // When
    let moved = still_gated(&gated, "ccc").expect_err("pushed during the gate");

    // Then
    assert!(moved.contains("moved to ccc"), "{moved}");
    assert_eq!(still_gated(&gated, "aaa"), Ok(()));
}
