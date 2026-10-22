//! The counting rule, the allow and generated lists, and the message.

use super::*;

fn rule(max_lines: usize) -> Rule {
    Rule {
        max_lines,
        generated: BTreeMap::new(),
        allow: BTreeMap::new(),
    }
}

#[test]
fn a_file_at_the_limit_passes() {
    // Given
    let counted = vec![("a.rs".to_owned(), 220)];

    // When / Then
    assert_eq!(over_limit(&counted, &rule(220)), []);
}

#[test]
fn a_file_one_over_the_limit_is_found_with_its_count() {
    // Given
    let counted = vec![("b.rs".to_owned(), 221)];

    // When
    let findings = over_limit(&counted, &rule(220));

    // Then
    assert_eq!(
        findings,
        [Finding {
            path: "b.rs".into(),
            lines: 221,
            allowed: None
        }]
    );
}

#[test]
fn generated_files_are_left_out() {
    // Given
    let mut rule = rule(10);
    rule.generated
        .insert("gen.rs".into(), "written by build.rs".into());
    let counted = vec![("gen.rs".to_owned(), 500), ("big.rs".to_owned(), 11)];

    // When
    let findings = over_limit(&counted, &rule);

    // Then
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].path, "big.rs");
}

#[test]
fn allowed_files_carry_their_reason() {
    // Given
    let mut rule = rule(10);
    rule.allow.insert("table.rs".into(), "a data table".into());
    let counted = vec![("table.rs".to_owned(), 300)];

    // When
    let findings = over_limit(&counted, &rule);

    // Then
    assert_eq!(findings[0].allowed.as_deref(), Some("a data table"));
}

#[test]
fn the_refusal_names_each_file_its_count_and_the_fix_and_prints_allowed_reasons() {
    // Given
    let findings = vec![
        Finding {
            path: "table.rs".into(),
            lines: 300,
            allowed: Some("a data table".into()),
        },
        Finding {
            path: "crates/x/big.rs".into(),
            lines: 241,
            allowed: None,
        },
    ];

    // When
    let error = report(&findings, &rule(220)).expect_err("one over");

    // Then
    assert_eq!(
        error,
        "allowed  table.rs (300 lines): a data table\n\
         error: crates/x/big.rs: 241 lines, over 220; fix: split it, or allow it in .config/factory.toml under [files.allow] as `crates/x/big.rs = \"<reason>\"`"
    );
}

#[test]
fn only_allowed_findings_pass_with_their_reasons_listed() {
    // Given
    let findings = vec![Finding {
        path: "table.rs".into(),
        lines: 300,
        allowed: Some("a data table".into()),
    }];

    // When / Then
    assert_eq!(
        report(&findings, &rule(220)),
        Ok(vec![
            "allowed  table.rs (300 lines): a data table".to_owned()
        ])
    );
    assert_eq!(report(&[], &rule(220)), Ok(vec![]));
}

#[test]
fn the_rule_reads_max_lines_and_the_two_lists_from_the_settings() {
    // Given
    let text = "[files]\nmax_lines = \"180\"\n[files.allow]\ncrates/a.rs = \"a table\"\n[files.generated]\nout.rs = \"by build.rs\"\n";
    let config = Config::parse(text).expect("valid");

    // When
    let rule = Rule::from_config(&config).expect("a rule");

    // Then
    assert_eq!(rule.max_lines, 180);
    assert_eq!(
        rule.allow.get("crates/a.rs").map(String::as_str),
        Some("a table")
    );
    assert_eq!(
        rule.generated.get("out.rs").map(String::as_str),
        Some("by build.rs")
    );
    assert_eq!(
        Rule::from_config(&Config::parse("[web]\npackage = \"x\"\n").expect("valid"))
            .expect("defaults")
            .max_lines,
        DEFAULT_MAX_LINES
    );
}

#[test]
fn a_max_lines_that_is_not_a_number_is_refused_naming_the_setting() {
    // Given
    let config = Config::parse("[files]\nmax_lines = \"many\"\n").expect("valid");

    // When
    let error = Rule::from_config(&config).expect_err("not a number");

    // Then
    assert_eq!(
        error,
        ".config/factory.toml: [files] max_lines = \"many\" is not a number"
    );
}

#[test]
fn an_allowance_naming_no_file_is_refused_as_stale() {
    // Given
    let mut rule = rule(220);
    rule.allow
        .insert("crates/gone.rs".into(), "was a table".into());
    rule.generated.insert("out.rs".into(), "by build.rs".into());
    let counted = vec![("crates/a.rs".to_owned(), 10)];

    // When
    let error = check_allowances(&counted, &rule).expect_err("stale");

    // Then
    assert_eq!(
        error,
        "error: [files.allow] `crates/gone.rs` names no file; stale allowance; remove it\n\
         error: [files.generated] `out.rs` names no file; stale allowance; remove it"
    );
}

#[test]
fn an_allowed_file_now_within_the_limit_is_refused_as_stale() {
    // Given
    let mut rule = rule(220);
    rule.allow.insert("crates/a.rs".into(), "a table".into());

    // When
    let error = check_allowances(&[("crates/a.rs".to_owned(), 200)], &rule).expect_err("stale");

    // Then
    assert_eq!(
        error,
        "error: [files.allow] `crates/a.rs` is 200 lines, within 220; stale allowance; remove it"
    );
}

#[test]
fn a_live_allowance_passes() {
    // Given
    let mut rule = rule(220);
    rule.allow.insert("crates/a.rs".into(), "a table".into());

    // When / Then
    assert_eq!(
        check_allowances(&[("crates/a.rs".to_owned(), 300)], &rule),
        Ok(())
    );
}
