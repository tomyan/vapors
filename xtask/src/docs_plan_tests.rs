use super::*;

const TABLE: &str = "\
### Runtime track (touche)

| Id | What | Owner | State | Depends on |
|---|---|---|---|---|
| R-43 | History | impl-runtime-2 | merged #43 (2026-10-25 17:16Z) | |
| P0-1..4a | Tokens | impl-runtime-3 | merged #54, #55, #58 | |
| P0-3c | Tagged layers | impl-runtime-3 | PR #64 (review fixes done at 7650efd; in the queue) | |
| P1a | Exact reductions | impl-runtime-3 | draft PR #72 on main; rebases once ENG-LIMITS merges | ENG-LIMITS |
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
    let text = "| Id | What | Owner | State | Depends on |\n|---|---|---|---|---|\n| A | x | y | done-ish | |\n";
    let (_, problems) = claims("docs/PLAN.md", text);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].starts_with("docs/PLAN.md:3: the State \"done-ish\" is in no shape"),
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

#[test]
fn a_table_with_a_state_column_and_another_header_is_refused() {
    let text = "| Id | What | Owner | State | Depends on | Notes |\n|---|---|---|---|---|---|\n| A | x | y | PR #999 | | |\n";
    let (_, problems) = claims("docs/PLAN.md", text);
    assert_eq!(
        problems,
        [format!(
            "docs/PLAN.md:1: a table with a State column is headed `| Id | What | Owner | State | Depends on | Notes |`, not `{HEADER}`; fix: use that header, or move the column out"
        )]
    );
}

#[test]
fn a_plan_that_names_no_pr_is_refused() {
    let none = read_plan("docs/PLAN.md", "# Plan\n\nNo tables.\n").1;
    assert_eq!(
        none,
        [format!(
            "docs/PLAN.md: no table is headed `{HEADER}`; fix: keep the work items in such a table"
        )]
    );
    let planned = format!("{HEADER}\n|---|---|---|---|---|\n| A | x | y | planned | |\n");
    assert_eq!(
        read_plan("docs/PLAN.md", &planned).1,
        [
            "docs/PLAN.md: its work-item tables name no PR, so nothing is compared with gh; fix: check that the States are read (the grammar at the top of the file)"
        ]
    );
}
