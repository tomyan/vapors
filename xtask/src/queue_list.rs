//! What the merge queue reads from GitHub: the open PRs (refused at the
//! list's limit, which may be partial), every comment of one through the
//! REST API's pages, and a PR as it is now. Each call runs within gh's
//! deadline (`queue_remote::gh`).

use crate::queue_remote::gh;

/// One open PR as gh lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    /// Its number.
    pub number: u64,
    /// Its head commit.
    pub head: String,
    /// Its head branch.
    pub branch: String,
    /// The branch it targets.
    pub base: String,
    /// Whether it is a draft.
    pub draft: bool,
    /// Whether it comes from a fork (isCrossRepository).
    pub fork: bool,
}

/// How many open PRs one list asks for; a list this long may be partial,
/// so it is refused rather than judged.
pub const LIST_LIMIT: usize = 100;

/// gh's `pr list --json` answer, oldest PR first.
///
/// # Errors
///
/// Not JSON, or a PR missing a field, naming both.
pub fn parse_list(json: &str) -> Result<Vec<Listed>, String> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("gh's PR list is not JSON ({e})"))?;
    let mut prs = value
        .as_array()
        .ok_or("gh's PR list is not a list")?
        .iter()
        .map(listed)
        .collect::<Result<Vec<_>, _>>()?;
    prs.sort_by_key(|pr| pr.number);
    Ok(prs)
}

fn listed(pr: &serde_json::Value) -> Result<Listed, String> {
    let number = pr["number"].as_u64().ok_or("a listed PR has no number")?;
    let missing = |field: &str| format!("#{number} has no {field} in gh's list");
    let text = |field: &str| {
        pr[field]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| missing(field))
    };
    let flag = |field: &str| pr[field].as_bool().ok_or_else(|| missing(field));
    Ok(Listed {
        number,
        head: text("headRefOid")?,
        branch: text("headRefName")?,
        base: text("baseRefName")?,
        draft: flag("isDraft")?,
        fork: flag("isCrossRepository")?,
    })
}

/// The open PRs of `repo` (`owner/name`), oldest first.
///
/// # Errors
///
/// gh's refusal, an answer in another shape, or a list at the limit.
pub fn open_prs(repo: &str) -> Result<Vec<Listed>, String> {
    let fields = "number,headRefOid,headRefName,baseRefName,isDraft,isCrossRepository";
    let limit = LIST_LIMIT.to_string();
    let args = [
        "pr", "list", "--repo", repo, "--state", "open", "--limit", &limit, "--json", fields,
    ];
    let prs = parse_list(&gh(&args)?)?;
    if prs.len() >= LIST_LIMIT {
        return Err(format!(
            "gh listed {LIST_LIMIT} open PRs, its limit, so the list may be partial; fix: raise LIST_LIMIT in queue_list.rs"
        ));
    }
    Ok(prs)
}

/// Every comment of PR `number`, oldest first, through the REST API's
/// pages (gh's GraphQL lists stop at 100).
///
/// # Errors
///
/// gh's refusal, or pages in another shape.
pub fn comments(repo: &str, number: u64) -> Result<Vec<String>, String> {
    let path = format!("repos/{repo}/issues/{number}/comments?per_page=100");
    let pages = gh(&["api", "--paginate", &path, "--jq", "[.[].body]"])?;
    parse_comment_pages(&pages).map_err(|e| format!("#{number}'s comments: {e}"))
}

/// gh's `--paginate --jq '[.[].body]'` output: one JSON array per page.
///
/// # Errors
///
/// A page that is not an array of strings.
pub fn parse_comment_pages(pages: &str) -> Result<Vec<String>, String> {
    let mut bodies = Vec::new();
    for page in serde_json::Deserializer::from_str(pages).into_iter::<Vec<String>>() {
        bodies.extend(page.map_err(|e| format!("a page is not a list of bodies ({e})"))?);
    }
    Ok(bodies)
}

/// PR `number` of `repo` as it is now: head, state and draft.
///
/// # Errors
///
/// gh's refusal, or an answer in another shape.
pub fn pr_now(repo: &str, number: u64) -> Result<crate::queue_ops::Now, String> {
    let json = gh(&[
        "pr",
        "view",
        &number.to_string(),
        "--repo",
        repo,
        "--json",
        "headRefOid,state,isDraft",
    ])?;
    let value: serde_json::Value = serde_json::from_str(&json)
        .map_err(|e| format!("gh's view of #{number} is not JSON ({e})"))?;
    let missing = |field: &str| format!("gh's view of #{number} has no {field}");
    let text = |field: &str| {
        value[field]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| missing(field))
    };
    Ok(crate::queue_ops::Now {
        head: text("headRefOid")?,
        state: text("state")?,
        draft: value["isDraft"]
            .as_bool()
            .ok_or_else(|| missing("isDraft"))?,
    })
}

#[cfg(test)]
#[path = "queue_list_tests.rs"]
mod tests;
