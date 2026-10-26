use super::*;

const TABLE: &str = "\
### Runtime track (touche)

| Id | What | Owner | State | Depends on |
|---|---|---|---|---|
| R-43 | History | impl-runtime-2 | merged #43 (2026-10-25 17:16Z) | |
| P0-1..4a | Tokens | impl-runtime-3 | merged #54, #55, #58 | |
| P0-3c | Tagged layers | impl-runtime-3 | PR #64 (review fixes done at 7650efd; in the queue) | |
| P1a | Exact reductions | impl-runtime-3 | draft PR #72 on main; rebases onto #73 | ENG-LIMITS |
| P0-4b | IndexedDB queue | unassigned | planned | P0-4a |
| X | Something | lead | pending Tom | |
";

#[test]
fn each_state_cell_claims_its_prs() {
    let (claims, problems) = claims("docs/PLAN.md", TABLE);
    assert_eq!(problems, Vec::<String>::new());
    assert_eq!(
        claims,
        [
            Claim {
                line: 5,
                repo: None,
                pr: 43,
                expect: Expect::Merged
            },
            Claim {
                line: 6,
                repo: None,
                pr: 54,
                expect: Expect::Merged
            },
            Claim {
                line: 6,
                repo: None,
                pr: 55,
                expect: Expect::Merged
            },
            Claim {
                line: 6,
                repo: None,
                pr: 58,
                expect: Expect::Merged
            },
            Claim {
                line: 7,
                repo: None,
                pr: 64,
                expect: Expect::Open
            },
            Claim {
                line: 8,
                repo: None,
                pr: 72,
                expect: Expect::Draft
            },
        ]
    );
}

#[test]
fn a_state_cell_in_an_unknown_shape_is_refused_naming_its_line() {
    let text = "| Id | What | Owner | State | Depends on |\n|---|---|---|---|---|\n| A | x | y | done-ish, see #12 | |\n";
    let (_, problems) = claims("docs/PLAN.md", text);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].starts_with("docs/PLAN.md:3: the State \"done-ish, see #12\" is in no shape"),
        "{problems:?}"
    );
}

#[test]
fn a_claim_that_gh_contradicts_is_said_with_both() {
    let claim = Claim {
        line: 7,
        repo: None,
        pr: 64,
        expect: Expect::Open,
    };
    let merged = Pr {
        state: "MERGED".into(),
        draft: false,
    };
    let said = contradiction("docs/PLAN.md", &claim, &merged).expect("contradicted");
    assert!(
        said.starts_with("docs/PLAN.md:7: #64 is listed as open, but gh says it is merged"),
        "{said}"
    );
    assert!(said.contains("fix: "), "{said}");
    let open = Pr {
        state: "OPEN".into(),
        draft: false,
    };
    assert_eq!(contradiction("docs/PLAN.md", &claim, &open), None);
    let draft = Pr {
        state: "OPEN".into(),
        draft: true,
    };
    let said = contradiction("docs/PLAN.md", &claim, &draft).expect("a draft is not open");
    assert!(said.contains("gh says it is an open draft"), "{said}");
}

#[test]
fn a_draft_claim_needs_an_open_draft_and_a_merged_claim_a_merge() {
    let draft = Claim {
        line: 8,
        repo: None,
        pr: 72,
        expect: Expect::Draft,
    };
    let ready = Pr {
        state: "OPEN".into(),
        draft: false,
    };
    assert!(
        contradiction("p", &draft, &ready)
            .expect("not a draft")
            .contains("listed as a draft")
    );
    let merged = Claim {
        line: 5,
        repo: None,
        pr: 43,
        expect: Expect::Merged,
    };
    let closed = Pr {
        state: "CLOSED".into(),
        draft: false,
    };
    assert!(
        contradiction("p", &merged, &closed)
            .expect("closed")
            .contains("gh says it is closed")
    );
}

#[test]
fn rows_outside_a_work_item_table_are_not_read() {
    let text = "| Track | Order |\n|---|---|\n| Runtime | #43 conversation history |\n";
    assert_eq!(claims("docs/PLAN.md", text), (Vec::new(), Vec::new()));
}

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
