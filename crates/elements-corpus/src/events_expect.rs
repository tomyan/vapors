//! An `.events` script's `expect` directive: what must hold once the
//! step before it is done (the forms are listed in `events_script`).

use crate::events_action::exactly;
use crate::events_script::{Expect, Script};

/// Reads `expect <fields>` at line `number` onto the script's last step.
pub(crate) fn expect(script: &mut Script, fields: &[&str], number: usize) -> Result<(), String> {
    let wanted = match fields.first().copied() {
        Some("event" | "no" | "value" | "checked") => 3,
        Some("selection") => 4,
        Some("focus" | "focus-visible") => 2,
        _ => fields.len(),
    };
    exactly("expect", fields, wanted)?;
    let expectation = match fields {
        ["event", kind, target] => Expect::Event { kind: (*kind).into(), target: (*target).into() },
        ["no", "event", kind] if !script.listen.iter().any(|t| t == kind) => {
            return Err(format!("`{kind}` is not listened for, so expecting none of it proves nothing"));
        }
        ["no", "event", kind] => Expect::NoEvent((*kind).into()),
        ["focus", target] => Expect::Focus((*target).into()),
        ["focus-visible", target] => Expect::FocusVisible((*target).into()),
        ["value", target, "\"\""] => Expect::Value { target: (*target).into(), value: String::new() },
        ["value", target, value] => Expect::Value { target: (*target).into(), value: (*value).into() },
        ["checked", target, state @ ("true" | "false")] => Expect::Checked { target: (*target).into(), checked: *state == "true" },
        ["checked", _, state] => return Err(format!("`expect checked` takes true or false, not `{state}`")),
        ["selection", target, start, end] => match (start.parse::<usize>(), end.parse::<usize>()) {
            (Ok(start), Ok(end)) if start <= end => Expect::Selection { target: (*target).into(), start, end },
            _ => return Err(format!("`expect selection` takes <start> <end>, whole numbers with start at most end, not `{start} {end}`")),
        },
        _ => return Err("`expect` takes event <type> <target>, no event <type>, focus <target>, focus-visible <target>, value <target> <text>, checked <target> true|false or selection <target> <start> <end>".into()),
    };
    let step = script
        .steps
        .last_mut()
        .ok_or("an expectation before any action")?;
    step.expect.push((expectation, number));
    Ok(())
}

#[cfg(test)]
#[path = "events_expect_tests.rs"]
mod tests;
