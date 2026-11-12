//! The Mount workload: its page mounts the rows it was given, measures
//! the mount, and the first frame's update sees every row.

use vapors_elements::Viewport;

use super::{Mount, row_data};
use crate::instructions_check::checked_counter;
use crate::run::Workload;

#[test]
fn row_data_numbers_its_rows_from_zero() {
    let rows = row_data(3);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[2], (2, "Row 2".to_owned(), "00014".to_owned()));
}

#[test]
fn a_mount_is_measured_and_its_page_updates_every_row() {
    // Given
    let mut mount = Mount::new(50, checked_counter().expect("perf events"));

    // When
    let page = mount.page();
    let viewport = Viewport {
        width: 640.0,
        height: 400.0,
        scale: 1.0,
    };
    page.doc.borrow_mut().update(viewport).expect("updates");

    // Then
    let visited = page.doc.borrow().update_counts().elements_visited;
    assert!(visited >= 50 * 4, "visited {visited}");
    let measured = mount.mounted[0];
    assert!(measured.instructions > 0 && measured.rss_after_kib >= measured.rss_before_kib);
}

#[test]
fn each_sample_after_the_first_mounts_a_fresh_page() {
    // Given
    let mut mount = Mount::new(10, checked_counter().expect("perf events"));

    // When
    mount.change(0);
    mount.change(1);

    // Then
    assert_eq!(mount.mounted.len(), 2);
}

#[test]
fn a_checked_frame_keeps_the_resident_set_after_it_and_the_peak() {
    // Given
    let mount = Mount::new(10, checked_counter().expect("perf events"));

    // When
    mount
        .check(vapors_platform::Next::Wait)
        .expect("a healthy frame");

    // Then
    let kept = mount.after_frame.borrow();
    assert_eq!(kept.len(), 1);
    assert!(kept[0].resident_kib > 0 && kept[0].peak_kib >= kept[0].resident_kib);
}

#[test]
fn a_mount_that_could_not_be_measured_fails_the_next_check_with_its_error() {
    // Given
    let mut mount = Mount::new(10, checked_counter().expect("perf events"));
    mount.failure = Some("the resident set cannot be read here".into());

    // When
    let checked = mount.check(vapors_platform::Next::Wait);

    // Then
    assert_eq!(checked, Err("the resident set cannot be read here".into()));
}
