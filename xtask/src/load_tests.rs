//! The machine's load, as the gate records it.

use super::*;

#[test]
fn the_one_minute_load_is_read_from_proc_loadavg() {
    assert_eq!(
        parse("38.12 41.07 39.90 7/1834 2291\n"),
        Ok("38.12".to_owned())
    );
}

#[test]
fn text_of_another_shape_is_refused_quoting_it() {
    for text in ["busy\n", "nan 1 1 1/1 1\n", "inf 1 1 1/1 1\n"] {
        let refusal = parse(text).expect_err(text);
        assert!(refusal.contains(text.trim()), "{refusal}");
    }
}

#[test]
fn a_step_line_names_the_load_at_its_start_and_end() {
    assert_eq!(
        line(&Ok("12.31".into()), &Ok("14.02".into())),
        "     load 12.31 at the start, 14.02 at the end"
    );
    assert!(line(&Err("no /proc".into()), &Ok("1.00".into())).contains("unreadable: no /proc"));
}
