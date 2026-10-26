//! The plan grammar's forms beyond the basic table: PRs in other
//! repositories, and notes on plain States.

use super::*;

fn one_row(state: &str) -> (Vec<Claim>, Vec<String>) {
    let text = format!("{HEADER}\n|---|---|---|---|---|\n| A | x | y | {state} | |\n");
    claims("docs/PLAN.md", &text)
}

/// The claims of a one-row table: repository (empty for the default), PR, state.
fn claimed(state: &str) -> Vec<(String, u64, Expect)> {
    let (claims, problems) = one_row(state);
    assert_eq!(problems, Vec::<String>::new(), "{state}");
    claims
        .into_iter()
        .map(|c| (c.repo.unwrap_or_default(), c.pr, c.expect))
        .collect()
}

#[test]
fn a_state_may_name_a_pr_in_another_repository() {
    assert_eq!(
        claimed("merged vapors #52, touche #66"),
        [
            ("vapors".to_owned(), 52, Expect::Merged),
            ("touche".to_owned(), 66, Expect::Merged)
        ]
    );
    assert_eq!(
        claimed("merged vapors #58 (2026-10-26 04:42Z)"),
        [("vapors".to_owned(), 58, Expect::Merged)]
    );
    assert_eq!(
        claimed("vapors PR #60 (CI green since 2026-10-25 20:52Z; not merged)"),
        [("vapors".to_owned(), 60, Expect::Open)]
    );
    assert_eq!(
        claimed("vapors draft PR #7"),
        [("vapors".to_owned(), 7, Expect::Draft)]
    );
}

#[test]
fn a_plain_state_may_carry_a_note_that_names_no_pr() {
    for state in [
        "planned, after PLAN merges",
        "planned; blocks 5-A1, 10-C1",
        "in progress (started 2026-10-26)",
        "parked, until vapors F4",
    ] {
        assert_eq!(claimed(state), [], "{state}");
    }
    for state in [
        "parked, draft #26 (no commit since 2026-10-23)",
        "planned, after #74 merges",
        "planned-ish",
        "PR (this one)",
    ] {
        let (_, problems) = one_row(state);
        assert_eq!(problems.len(), 1, "{state}: {problems:?}");
    }
}

#[test]
fn a_contradiction_in_another_repository_names_it() {
    let claim = Claim {
        line: 9,
        repo: Some("vapors".into()),
        pr: 60,
        expect: Expect::Open,
    };
    let merged = Pr {
        state: "MERGED".into(),
        draft: false,
    };
    let said = contradiction("docs/PLAN.md", &claim, &merged).expect("contradicted");
    assert!(
        said.starts_with("docs/PLAN.md:9: vapors #60 is listed as open, but gh says it is merged"),
        "{said}"
    );
}
