//! A signal or memo binding passed to a child's prop converts with
//! `into`, so a prop typed `ReadSignal<T>` takes it; any other
//! expression (a closure, a number, a call) is passed unchanged,
//! since `into` would break a closure or an open float type.

use crate::common_codegen as codegen;

use codegen::{assert_has, assert_lacks, generated_with};

const REASON: &str = "a prop that is a signal or memo binding converts into the child's prop type (ReadSignal or Signal); the compiler cannot see when the types already match";

#[test]
fn a_signal_binding_converts_into_the_childs_prop() {
    let script = "let tilt: Signal<f32> = Signal::new(0.0);";

    let code = generated_with(script, "<Greeting angle={tilt} />");

    assert_has(
        &code,
        &format!(
            "#[allow(clippy::useless_conversion, reason = {REASON:?})]\nlet greeting0_angle = tilt.into();"
        ),
    );
}

#[test]
fn a_memo_binding_converts_too() {
    let script =
        "let a: Signal<i32> = Signal::new(0);\nlet b: Memo<i32> = Memo::new(move || a.get());";

    let code = generated_with(script, "<Greeting n={b} />");

    assert_has(&code, "let greeting0_n = b.into();");
}

#[test]
fn a_prop_of_signal_type_converts() {
    let script = "#[prop]\nlet lens: Signal<i32> = Signal::new(0);";

    let code = generated_with(script, "<Greeting n={lens} />");

    assert_has(&code, "let greeting0_n = lens.into();");
}

#[test]
fn other_expressions_pass_unchanged() {
    let script = "let who: String = String::new();";
    let tag = "<Greeting size={1.0 + 2.0} name={who.clone()} on={move |v: i32| drop(v)} />";

    let code = generated_with(script, tag);

    assert_has(&code, "let greeting0_size = 1.0 + 2.0;");
    assert_has(&code, "let greeting0_name = who.clone();");
    assert_lacks(&code, "useless_conversion");
}

#[test]
fn a_read_signal_binding_is_a_field_that_reads_in_the_template() {
    let script = "let a: Signal<i32> = Signal::new(0);\nlet twice: ReadSignal<i32> = Signal::derived(move || a.get() * 2);";

    let code = generated_with(script, "<p>{twice}</p>");

    assert_has(&code, "pub twice: ReadSignal<i32>,");
    assert_has(&code, "twice.get()");
}
