//! What a tailscale mount serves now, read from `tailscale serve status
//! --json`, so `cargo xtask serve` and `dev` never replace another
//! target without being told to (`--replace`).

use crate::serve_config::Mount;

/// What `status` says is served at `mount` on `host`: a `Path`, a
/// `Proxy` or another handler, as written; `None` when nothing is.
///
/// # Errors
///
/// `status` is not the JSON `tailscale serve status --json` writes.
pub fn served_at(status: &str, host: &str, mount: &Mount) -> Result<Option<String>, String> {
    let json: serde_json::Value = serde_json::from_str(status)
        .map_err(|e| format!("`tailscale serve status --json` is not JSON: {e}"))?;
    let handler = &json["Web"][format!("{host}:{}", mount.https)]["Handlers"][&mount.path];
    Ok(match handler {
        serde_json::Value::Null => None,
        serde_json::Value::Object(fields) => fields
            .iter()
            .next()
            .map(|(kind, to)| format!("{kind} {}", to.as_str().unwrap_or_default())),
        other => Some(other.to_string()),
    })
}

/// Refuse to replace `current` with `wanted` unless `replace` is set.
///
/// # Errors
///
/// Another target is mounted there; the refusal names it and the fix.
pub fn refuse_other(
    current: Option<&str>,
    wanted: &str,
    mount: &Mount,
    replace: bool,
) -> Result<(), String> {
    match current {
        Some(other) if other != wanted && !replace => Err(format!(
            "https {} {} already serves {other}; fix: rerun with --replace to serve {wanted} there instead",
            mount.https, mount.path
        )),
        _ => Ok(()),
    }
}

#[cfg(test)]
#[path = "serve_mount_tests.rs"]
mod tests;
