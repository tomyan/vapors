use super::*;

#[test]
fn a_pin_file_s_first_line_that_is_not_a_comment_is_the_pin() {
    let text = "# The vapors commit CI builds against.\n# more\n76faa5a2aacd41d358974dd6e32b1c3802ba591e\n";
    assert_eq!(
        pin_in(text, "p"),
        Ok("76faa5a2aacd41d358974dd6e32b1c3802ba591e".to_owned())
    );
}

#[test]
fn a_pin_file_with_no_sha_is_refused_naming_it() {
    let refused = pin_in("# only comments\n", ".config/vapors-rev").expect_err("no pin");
    assert!(refused.contains(".config/vapors-rev"), "{refused}");
}

#[test]
fn only_the_expected_sibling_is_followed() {
    let entry = |k: &str, v: &str| std::collections::BTreeMap::from([(k.to_owned(), v.to_owned())]);
    assert_eq!(
        allowed(&entry("../vapors", ".config/vapors-rev")).map(|v| v.len()),
        Ok(1)
    );
    assert_eq!(allowed(&std::collections::BTreeMap::new()), Ok(Vec::new()));
    for (k, v) in [
        ("../../../../projects/vapors", ".config/vapors-rev"),
        ("../vapors", "other-rev"),
        ("../other", ".config/vapors-rev"),
    ] {
        let refused = allowed(&entry(k, v)).expect_err(k);
        assert!(
            refused.contains(k) && refused.contains("fix: "),
            "{refused}"
        );
    }
}
