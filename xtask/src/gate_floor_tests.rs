//! The gate's floors on free inodes in the temp dir and free space
//! under target/.

use std::path::Path;

use super::*;

const PLENTY: Free = Free {
    inodes: Some(INODE_FLOOR * 10),
    bytes: DISK_FLOOR * 10,
};

#[test]
fn stat_output_reads_as_free_inodes_and_bytes() {
    // When: total and free inodes, free blocks for users, block size
    let free = parse("1048576 466937 26214400 4096\n", Path::new("/tmp")).expect("parsed");

    // Then
    assert_eq!(
        free,
        Free {
            inodes: Some(466_937),
            bytes: 26_214_400 * 4096
        }
    );
}

#[test]
fn stat_output_of_another_shape_is_refused_naming_the_path_and_the_text() {
    // When
    let refusal = parse("466937 lots\n", Path::new("/tmp")).expect_err("refused");

    // Then
    assert!(
        refusal.contains("/tmp") && refusal.contains("466937 lots"),
        "{refusal}"
    );
}

#[test]
fn above_both_floors_the_gate_goes_on() {
    assert_eq!(
        check((Path::new("/tmp"), PLENTY), (Path::new("target"), PLENTY)),
        Ok(())
    );
}

#[test]
fn too_few_free_inodes_in_the_temp_dir_is_refused_naming_the_fix() {
    // Given
    let low = Free {
        inodes: Some(INODE_FLOOR - 1),
        ..PLENTY
    };

    // When
    let refusal = check((Path::new("/tmp"), low), (Path::new("target"), PLENTY)).expect_err("low");

    // Then
    assert!(
        refusal.contains("/tmp has") && refusal.contains("free inodes"),
        "{refusal}"
    );
    assert!(refusal.contains("fix: "), "{refusal}");
}

#[test]
fn too_little_free_space_under_target_is_refused_naming_the_fix() {
    // Given
    let low = Free {
        bytes: DISK_FLOOR - 1,
        ..PLENTY
    };

    // When
    let refusal = check((Path::new("/tmp"), PLENTY), (Path::new("target"), low)).expect_err("low");

    // Then
    assert!(
        refusal.contains("target has") && refusal.contains("GB free"),
        "{refusal}"
    );
    assert!(refusal.contains("fix: "), "{refusal}");
}

#[test]
fn stat_reads_this_machines_root_filesystem() {
    // When
    let free = free(Path::new("/")).expect("stat -f /");

    // Then
    assert!(
        free.inodes.is_some_and(|n| n > 0) && free.bytes > 0,
        "{free:?}"
    );
}

#[test]
fn a_filesystem_without_an_inode_count_has_no_inode_floor() {
    // Given: btrfs reports 0 total and 0 free inodes
    let free = parse("0 0 26214400 4096\n", Path::new("/tmp")).expect("parsed");

    // When / Then
    assert_eq!(free.inodes, None);
    assert_eq!(
        check((Path::new("/tmp"), free), (Path::new("target"), PLENTY)),
        Ok(())
    );
}

#[test]
fn target_is_the_workspace_before_a_build_and_a_dangling_link_is_refused() {
    // Given
    let workspace = crate::scratch::dir("floor-target");

    // When: no target yet, then target linked to nothing
    let before_build = target_dir(&workspace);
    std::os::unix::fs::symlink(workspace.join("gone"), workspace.join("target")).expect("link");
    let dangling = target_dir(&workspace).expect_err("refused");

    // Then
    assert_eq!(before_build, Ok(workspace.to_path_buf()));
    assert!(
        dangling.contains("does not exist") && dangling.contains("fix: "),
        "{dangling}"
    );
}
