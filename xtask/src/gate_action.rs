//! A local composite action (`uses: ./path`) as the gate reads it: its
//! steps, and its inputs from the calling step's `with:` over the
//! action's defaults, substituted for `${{ inputs.<name> }}` in a script.
//! Any other expression, an unknown input, or a missing required one is
//! refused.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::gate_workflow::check_keys;

const ACTION_KEYS: [&str; 6] = ["name", "description", "author", "inputs", "outputs", "runs"];

/// The steps and resolved inputs of a composite action.
pub struct Composite {
    /// `runs.steps`, in order.
    pub steps: Vec<Value>,
    /// Each input's value for this call.
    pub inputs: BTreeMap<String, String>,
}

/// The composite action at `path` (its `action.yml`'s text in `text`),
/// called with `with`.
pub fn composite(text: &str, path: &str, with: Option<&Value>) -> Result<Composite, String> {
    let doc = crate::gate_yaml::parse(text, path)?;
    check_keys(&doc, &ACTION_KEYS, path)?;
    let runs = doc
        .get("runs")
        .ok_or_else(|| format!("{path}: no `runs:`"))?;
    check_keys(runs, &["using", "steps"], &format!("{path}: runs"))?;
    if runs.get("using").and_then(Value::as_str) != Some("composite") {
        return Err(format!(
            "{path}: not a composite action, which is the only kind the gate runs"
        ));
    }
    let steps = runs
        .get("steps")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{path}: no `runs.steps:`"))?
        .clone();
    Ok(Composite {
        steps,
        inputs: inputs(&doc, path, with)?,
    })
}

/// Each declared input: the caller's `with:` value, else its default.
fn inputs(
    doc: &Value,
    path: &str,
    with: Option<&Value>,
) -> Result<BTreeMap<String, String>, String> {
    let given = given(with, path)?;
    let declared = doc
        .get("inputs")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if let Some(unknown) = given
        .keys()
        .find(|name| !declared.contains_key(name.as_str()))
    {
        return Err(format!(
            "{path}: `with: {unknown}`, which the action does not declare"
        ));
    }
    let mut resolved = BTreeMap::new();
    for (name, input) in &declared {
        let name = name.as_str();
        let default = input.get("default").and_then(Value::as_str);
        let value = given
            .get(name)
            .map(String::as_str)
            .or(default)
            .ok_or_else(|| format!("{path}: input `{name}` has no value and no default"))?;
        if value.contains("${{") {
            return Err(format!(
                "{path}: input `{name}` is an expression, which the gate cannot evaluate"
            ));
        }
        resolved.insert(name.to_owned(), value.to_owned());
    }
    Ok(resolved)
}

fn given(with: Option<&Value>, path: &str) -> Result<BTreeMap<String, String>, String> {
    let Some(with) = with else {
        return Ok(BTreeMap::new());
    };
    let map = with
        .as_object()
        .ok_or_else(|| format!("{path}: `with:` is not a mapping"))?;
    map.iter()
        .map(|(name, value)| {
            let name = name.as_str();
            let value = match value {
                Value::String(text) => text.clone(),
                Value::Number(number) => number.to_string(),
                Value::Bool(flag) => flag.to_string(),
                _ => {
                    return Err(format!(
                        "{path}: `with: {name}` is not a string, number or bool"
                    ));
                }
            };
            Ok((name.to_owned(), value))
        })
        .collect()
}

/// `step` with each `${{ inputs.<name> }}` in its `env:` values replaced,
/// so the env check sees the values the action is called with.
pub fn with_inputs_in_env(
    step: &Value,
    inputs: &BTreeMap<String, String>,
    label: &str,
) -> Result<Value, String> {
    let mut step = step.clone();
    if let Some(env) = step.get_mut("env").and_then(Value::as_object_mut) {
        for value in env.values_mut() {
            if let Value::String(text) = value {
                *text = substitute(text, inputs, label)?;
            }
        }
    }
    Ok(step)
}

/// `run` with each `${{ inputs.<name> }}` replaced by its value; any
/// other expression is left for the script check to refuse.
pub fn substitute(
    run: &str,
    inputs: &BTreeMap<String, String>,
    label: &str,
) -> Result<String, String> {
    let mut out = String::new();
    let mut rest = run;
    while let Some(start) = rest.find("${{") {
        let end = rest[start..]
            .find("}}")
            .ok_or_else(|| format!("{label}: an unclosed `${{{{`"))?
            + start;
        let inner = rest[start + 3..end].trim();
        out.push_str(&rest[..start]);
        match inner
            .strip_prefix("inputs.")
            .and_then(|name| inputs.get(name))
        {
            Some(value) => out.push_str(value),
            None => out.push_str(&rest[start..end + 2]),
        }
        rest = &rest[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}
