use super::*;
use crate::queue_remote::Listed;

fn pr(number: u64, branch: &str, base: &str) -> Listed {
    Listed {
        number,
        head: format!("h{number}"),
        branch: branch.into(),
        base: base.into(),
        draft: false,
        comments: Vec::new(),
    }
}

#[test]
fn the_prs_stacked_on_a_merged_branch_are_found() {
    let prs = [
        pr(67, "dwim/a", "main"),
        pr(68, "dwim/b", "dwim/a"),
        pr(69, "web/c", "main"),
        pr(70, "dwim/d", "dwim/a"),
    ];
    assert_eq!(stacked_on(&prs, "dwim/a"), [68, 70]);
    assert_eq!(stacked_on(&prs, "web/c"), Vec::<u64>::new());
}
