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
