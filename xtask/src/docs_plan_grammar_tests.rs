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
    for state in ["planned, after #74 merges", "planned-ish", "PR (this one)"] {
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

#[test]
fn every_pr_a_state_names_is_a_claim() {
    let vapors = |pr, expect| ("vapors".to_owned(), pr, expect);
    let here = |pr, expect| (String::new(), pr, expect);
    assert_eq!(
        claimed("vapors PR #63 (fixes pushed); the disk follow-up is vapors PR #70"),
        [vapors(63, Expect::Open), vapors(70, Expect::Open)]
    );
    assert_eq!(
        claimed("parked; draft PR #26 (no commit since 2026-10-23)"),
        [here(26, Expect::Draft)]
    );
    assert_eq!(
        claimed("parked, draft #26 (no commit)"),
        [here(26, Expect::Draft)]
    );
    assert_eq!(claimed("PR #64 (draft)"), [here(64, Expect::Draft)]);
    assert_eq!(
        claimed("merged #54, #55 (2026-10-25)"),
        [here(54, Expect::Merged), here(55, Expect::Merged)]
    );
}

#[test]
fn a_pr_named_where_its_state_cannot_be_read_is_refused_naming_it() {
    for (state, pr) in [
        (
            "draft PR #72 on main; rebases onto #73 when #73 merges",
            "#73",
        ),
        ("merged #12 (superseded by #13)", "#13"),
        ("planned, after #74 merges", "#74"),
    ] {
        let (_, problems) = one_row(state);
        assert_eq!(problems.len(), 1, "{state}: {problems:?}");
        assert!(
            problems[0].contains(&format!("names {pr} where this check cannot read its state; fix: write it as PR {pr}, draft PR {pr} or merged {pr}")),
            "{problems:?}"
        );
    }
}
