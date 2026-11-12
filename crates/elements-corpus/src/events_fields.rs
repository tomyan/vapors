//! The fields of an `.events` script's setup directives: `at`'s size,
//! `listen`'s types, `page`'s file and `project`'s roles (`events_script`).

use vapors_elements::chromium_name_for_aria;

/// `page`'s file: a path under `fixtures/`, never above it.
pub(crate) fn page(file: &str) -> Result<String, String> {
    if file.starts_with('/') || file.split('/').any(|part| part == "..") {
        return Err(format!(
            "`page {file}`: a path under fixtures/, not above it or absolute"
        ));
    }
    Ok(file.to_owned())
}

pub(crate) fn at(rest: &str) -> Result<(u32, u32), String> {
    let size = rest.split_whitespace().next().unwrap_or_default();
    let parsed = size
        .split_once('x')
        .and_then(|(w, h)| Some((w.parse::<u32>().ok()?, h.parse::<u32>().ok()?)))
        .filter(|(w, h)| *w > 0 && *h > 0);
    parsed.ok_or_else(|| "`at` takes WxH".to_owned())
}

pub(crate) fn listen(rest: &str) -> Result<Vec<String>, String> {
    let types: Vec<String> = rest.split_whitespace().map(str::to_owned).collect();
    if types.is_empty() {
        return Err("`listen` takes event types".into());
    }
    Ok(types)
}

/// `project`'s roles, refusing one named as ARIA names it where
/// Chromium's tree uses another: it would match nothing on either side.
pub(crate) fn project_roles(fields: &[&str]) -> Result<Vec<String>, String> {
    if let Some((role, chromium)) = fields
        .iter()
        .find_map(|f| chromium_name_for_aria(f).map(|c| (f, c)))
    {
        return Err(format!(
            "`project` role `{role}` is ARIA's name; use `{chromium}`, Chromium's name"
        ));
    }
    Ok(fields.iter().map(|f| (*f).to_owned()).collect())
}
