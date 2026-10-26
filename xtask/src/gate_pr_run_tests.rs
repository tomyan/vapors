//! `run_with`'s sequence, driven by a recording fake of its effects.

use std::cell::RefCell;

use super::*;

const HEAD: &str = "b26d5aa3cf0d40373d621158d8231e3555598eeb";
const OTHER: &str = "0000000000000000000000000000000000000000";

struct Fake {
    heads: RefCell<Vec<&'static str>>,
    state: &'static str,
    local: &'static str,
    ran: Result<(Ended, String), String>,
    post: Result<(), String>,
    secrets: Vec<(String, String)>,
    calls: RefCell<Vec<String>>,
}

fn fake() -> Fake {
    Fake {
        heads: RefCell::new(vec![HEAD, HEAD]),
        state: "OPEN",
        local: HEAD,
        ran: Ok((Ended::Code(0), format!("ok   ci\ngate: pass {HEAD}\n"))),
        post: Ok(()),
        secrets: Vec::new(),
        calls: RefCell::new(Vec::new()),
    }
}

impl Effects for Fake {
    fn find(&self, _number: Option<u64>) -> Result<Pr, String> {
        self.calls.borrow_mut().push("find".into());
        let head = self.heads.borrow_mut().remove(0);
        Ok(Pr {
            number: 61,
            head: head.into(),
            state: self.state.into(),
        })
    }
    fn local_head(&self) -> Result<String, String> {
        self.calls.borrow_mut().push("local".into());
        Ok(self.local.into())
    }
    fn gate(&self) -> Result<Ran, String> {
        self.calls.borrow_mut().push("gate".into());
        self.ran
            .clone()
            .map(|(ended, output)| Ran { ended, output })
    }
    fn post(&self, _number: u64, body: &str) -> Result<(), String> {
        self.calls
            .borrow_mut()
            .push(format!("post {}", body.lines().next().unwrap_or("")));
        self.post.clone()
    }
    fn secrets(&self) -> Vec<(String, String)> {
        self.secrets.clone()
    }
}

fn calls(fake: &Fake) -> Vec<String> {
    fake.calls.borrow().clone()
}

#[test]
fn a_pass_for_the_pr_s_head_is_posted_after_the_head_is_read_again() {
    let fake = fake();
    assert_eq!(run_with(&fake, Some(61)), Ok(Ended::Code(0)));
    assert_eq!(
        calls(&fake),
        [
            "find",
            "local",
            "gate",
            "find",
            &format!("post Gate output for {HEAD} (`cargo xtask gate --pr 61`, exit 0):")
        ]
    );
}

#[test]
fn a_pass_for_another_commit_is_not_posted_naming_both() {
    // HEAD moved while the gate waited for its lock: it gated OTHER.
    let fake = Fake {
        ran: Ok((Ended::Code(0), format!("gate: pass {OTHER}\n"))),
        ..fake()
    };
    let refused = run_with(&fake, Some(61)).expect_err("an ungated head");
    assert!(
        refused.contains(&format!("the gate passed {OTHER}, not #61's head {HEAD}")),
        "{refused}"
    );
    assert!(
        !calls(&fake).iter().any(|c| c.starts_with("post")),
        "{:?}",
        calls(&fake)
    );
}

#[test]
fn a_head_mismatch_is_refused_before_the_gate_runs() {
    let fake = Fake {
        local: OTHER,
        ..fake()
    };
    assert!(run_with(&fake, Some(61)).is_err());
    assert_eq!(calls(&fake), ["find", "local"]);
}

#[test]
fn a_pr_that_is_not_open_is_refused_before_the_gate_runs() {
    let fake = Fake {
        state: "MERGED",
        ..fake()
    };
    let refused = run_with(&fake, Some(61)).expect_err("merged");
    assert!(refused.contains("MERGED"), "{refused}");
    assert_eq!(calls(&fake), ["find"]);
}

#[test]
fn a_head_that_moved_during_the_gate_is_not_posted() {
    let fake = Fake {
        heads: RefCell::new(vec![HEAD, OTHER]),
        ..fake()
    };
    assert!(run_with(&fake, Some(61)).is_err());
    assert_eq!(calls(&fake), ["find", "local", "gate", "find"]);
}

#[test]
fn a_failed_gate_is_posted_with_its_exit_and_returned() {
    let fake = Fake {
        ran: Ok((Ended::Code(1), "FAIL   3/24 steps\n".into())),
        ..fake()
    };
    assert_eq!(run_with(&fake, Some(61)), Ok(Ended::Code(1)));
    assert!(calls(&fake).last().is_some_and(|c| c.ends_with("exit 1):")));
}

#[test]
fn a_failed_post_says_what_the_gate_did_and_the_fix() {
    let fake = Fake {
        post: Err("gh refused".into()),
        ..fake()
    };
    let refused = run_with(&fake, Some(61)).expect_err("not posted");
    assert!(
        refused.contains("the gate exited 0") && refused.contains("gh refused"),
        "{refused}"
    );
    assert!(
        refused.contains("fix: gh pr comment 61 --body-file target/gate/pr_comment.md"),
        "{refused}"
    );
}

#[test]
fn a_body_holding_a_secret_is_not_posted_naming_the_variable() {
    let secret = "ghp_abcdefghijklmnop".to_owned();
    let fake = Fake {
        ran: Ok((Ended::Code(1), format!("token {secret} printed\n"))),
        secrets: vec![("GITHUB_TOKEN".into(), secret.clone())],
        ..fake()
    };
    let refused = run_with(&fake, Some(61)).expect_err("a secret");
    assert!(
        refused.contains("GITHUB_TOKEN") && !refused.contains(&secret),
        "{refused}"
    );
    assert!(!calls(&fake).iter().any(|c| c.starts_with("post")));
}
