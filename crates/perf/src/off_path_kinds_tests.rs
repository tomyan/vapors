//! Every off-path page mounts without a refusal, each kind runs at
//! every size with each of its changes, and the names are distinct.

use super::{Kind, SIZES, off_path_workloads};
use crate::off_path_page::UnitPage;

fn names() -> Vec<String> {
    off_path_workloads()
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}

#[test]
fn every_kind_mounts_without_a_refusal() {
    for kind in Kind::ALL {
        if let Err(error) = UnitPage::new(&kind.shape(), 20) {
            panic!("{kind:?}: {error}");
        }
    }
}

#[test]
fn every_inner_scroller_moves_sideways_by_a_scroll_step() {
    for kind in Kind::ALL {
        let page = UnitPage::new(&kind.shape(), 20).expect("mounts");
        for unit in page.units.iter().filter_map(|unit| unit.inner) {
            let moved = page.doc.borrow_mut().scroll_node_by(unit, [24.0, 0.0]);
            assert!(moved, "{kind:?}: {unit:?} has no sideways scroll range");
        }
    }
}

#[test]
fn each_kind_runs_at_every_size_with_each_of_its_changes() {
    let names = names();
    for kind in Kind::ALL {
        for count in SIZES {
            for change in kind.changes() {
                let name = format!("{count} {}, {change:?}", kind.noun());
                assert!(names.contains(&name), "{name} in {names:?}");
            }
        }
    }
}

#[test]
fn no_name_is_given_twice_or_says_rows() {
    let mut names = names();
    assert!(names.iter().all(|name| !name.contains("rows")), "{names:?}");
    let count = names.len();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), count);
}
