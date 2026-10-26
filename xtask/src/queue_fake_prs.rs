//! PRs and clears for the fake's tables (`queue_fake.rs`).

use crate::queue_list::Listed;

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
