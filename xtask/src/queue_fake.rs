//! A recording fake of the queue's effects (`queue_ops::Ops`), for the
//! pass's tests: answers per PR from small tables, and every call noted.

use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::time::Duration;

use crate::gate_pr::{Ended, comment};
use crate::queue_decide::{Clear, Gate};
use crate::queue_list::Listed;
use crate::queue_ops::{Gated, Now, Ops};

/// A full-SHA clear signed `role`.
pub fn clear(role: &str) -> String {
    format!("review: clear {} ({role})", "c".repeat(40))
}

/// A listed PR on main, its branch `b<number>`, head `h<number>`.
pub fn listed(number: u64) -> Listed {
    Listed {
        number,
        head: format!("h{number}"),
        branch: format!("b{number}"),
        base: "main".into(),
        draft: false,
        fork: false,
    }
}

/// The fake's tables and its record of calls.
#[derive(Default)]
pub struct Fake {
    pub prs: Vec<Listed>,
    pub comments: RefCell<BTreeMap<u64, Vec<String>>>,
    /// Heads `now` reads, in turn, per PR; the listed head once empty.
    pub heads: RefCell<BTreeMap<u64, VecDeque<String>>>,
    /// Answers of `contains_main`, in turn, per head; true once empty.
    pub contains: RefCell<BTreeMap<String, VecDeque<bool>>>,
    pub touched: Vec<String>,
    pub rebased: Option<String>,
    /// Whether a gate posts its pass for the head it ran on.
    pub gate_posts: bool,
    /// The gate's error (killed at its deadline, could not start).
    pub gate_fails: Option<String>,
    pub gate_exit: i32,
    pub gate_tail: String,
    pub merge_fails: Vec<u64>,
    pub checks: BTreeMap<u64, Gate>,
    /// A PR that gets `queue: hold` once its comments were read once (a
    /// hold posted between judging and acting).
    pub hold_after_first_read: Option<u64>,
    pub calls: RefCell<Vec<String>>,
}

impl Fake {
    fn note(&self, call: String) {
        self.calls.borrow_mut().push(call);
    }

    /// The calls made, in order.
    pub fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }

    /// Whether any call starts with `prefix`.
    pub fn called(&self, prefix: &str) -> bool {
        self.calls.borrow().iter().any(|c| c.starts_with(prefix))
    }
}

impl Ops for Fake {
    fn open_prs(&self) -> Result<Vec<Listed>, String> {
        Ok(self.prs.clone())
    }
    fn comments(&self, number: u64) -> Result<Vec<String>, String> {
        let read = self
            .comments
            .borrow()
            .get(&number)
            .cloned()
            .unwrap_or_default();
        if self.hold_after_first_read == Some(number) {
            self.comments
                .borrow_mut()
                .entry(number)
                .or_default()
                .push("queue: hold".into());
        }
        Ok(read)
    }
    fn now(&self, number: u64) -> Result<Now, String> {
        let listed = self
            .prs
            .iter()
            .find(|p| p.number == number)
            .ok_or("no such PR")?;
        let head = self
            .heads
            .borrow_mut()
            .get_mut(&number)
            .and_then(VecDeque::pop_front);
        Ok(Now {
            head: head.unwrap_or_else(|| listed.head.clone()),
            state: "OPEN".into(),
            draft: false,
        })
    }
    fn fetch(&self, refs: &[&str]) -> Result<(), String> {
        self.note(format!("fetch {}", refs.join(" ")));
        Ok(())
    }
    fn contains_main(&self, head: &str) -> Result<bool, String> {
        Ok(self
            .contains
            .borrow_mut()
            .get_mut(head)
            .and_then(VecDeque::pop_front)
            .unwrap_or(true))
    }
    fn clear_holds(&self, _head: &str, _cleared: &str) -> Result<Clear, String> {
        Ok(Clear::Holds)
    }
    fn touched(&self, _head: &str) -> Result<Vec<String>, String> {
        Ok(self.touched.clone())
    }
    fn rebased(&self, head: &str) -> Result<Option<String>, String> {
        self.note(format!("rebase {head}"));
        Ok(self.rebased.clone())
    }
    fn push(&self, branch: &str, known: &str, new: &str) -> Result<(), String> {
        self.note(format!("push {branch} {known} {new}"));
        Ok(())
    }
    fn gate(&self, number: u64, head: &str) -> Result<Gated, String> {
        if let Some(why) = &self.gate_fails {
            self.note(format!("gate {number} {head}"));
            return Err(why.clone());
        }
        self.note(format!("gate {number} {head}"));
        if self.gate_posts {
            let body = comment(head, number, Ended::Code(0), &format!("gate: pass {head}"));
            self.comments
                .borrow_mut()
                .entry(number)
                .or_default()
                .push(body);
        }
        Ok(Gated {
            ended: Ended::Code(self.gate_exit),
            tail: self.gate_tail.clone(),
        })
    }
    fn checks(&self, number: u64) -> Result<Gate, String> {
        Ok(self.checks.get(&number).copied().unwrap_or(Gate::Passed))
    }
    fn update_branch(&self, number: u64) -> Result<(), String> {
        self.note(format!("update {number}"));
        Ok(())
    }
    fn merge(&self, number: u64, head: &str) -> Result<(), String> {
        self.note(format!("merge {number} {head}"));
        if self.merge_fails.contains(&number) {
            Err("gh refused: a conflict".into())
        } else {
            Ok(())
        }
    }
    fn retarget(&self, number: u64) -> Result<(), String> {
        self.note(format!("retarget {number}"));
        Ok(())
    }
    fn delete_branch(&self, branch: &str) -> Result<(), String> {
        self.note(format!("delete {branch}"));
        Ok(())
    }
    fn serve(&self) -> Result<(), String> {
        self.note("serve".into());
        Ok(())
    }
    fn wait(&self, duration: Duration) {
        self.note(format!("wait {}s", duration.as_secs()));
    }
    fn now_secs(&self) -> u64 {
        1_000
    }
}
