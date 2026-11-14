use super::*;

#[test]
fn every_longhand_is_in_exactly_one_scene_group() {
    for id in PropertyId::all() {
        // When
        let groups = GROUPS
            .iter()
            .filter(|group| group.names().any(|name| name == id.name()))
            .count();

        // Then
        assert_eq!(
            groups,
            1,
            "`{}` is in {groups} groups of crates/css/src/property/scene.rs; place it in exactly one",
            id.name()
        );
    }
}

#[test]
fn every_name_in_a_scene_group_is_a_longhand() {
    for name in GROUPS.iter().flat_map(Group::names) {
        assert!(
            PropertyId::from_name(name).is_some(),
            "`{name}` in crates/css/src/property/scene.rs is not a longhand of the property table; correct or remove it"
        );
    }
}

#[test]
fn each_scene_tag_reads_the_longhands_of_section_4_2() {
    // Given: docs/plans/css-for-3d.md, section 4.2, less custom properties
    let drawn = ["pointer-events", "visibility"];
    let mesh_motion = PropertyId::all()
        .map(PropertyId::name)
        .filter(|name| name.starts_with("transition-") || name.starts_with("animation-"));
    let mesh: Vec<&str> = ["color", "cursor", "opacity", "pointer-events", "visibility"]
        .into_iter()
        .chain(mesh_motion)
        .collect();
    let expected = [
        (SceneTag::Scene, drawn.to_vec()),
        (SceneTag::Group, drawn.to_vec()),
        (SceneTag::Mesh, mesh),
        (SceneTag::Camera, Vec::new()),
        (SceneTag::Decal, drawn.to_vec()),
    ];

    for (tag, mut reads) in expected {
        // When
        let mut read: Vec<&str> = PropertyId::all()
            .filter(|&id| tag.reads(id))
            .map(PropertyId::name)
            .collect();

        // Then
        read.sort_unstable();
        reads.sort_unstable();
        assert_eq!(read, reads, "<{}>", tag.name());
    }
}

#[test]
fn a_scene_tag_is_found_by_its_element_tag_and_no_other_is() {
    // Then
    for tag in SceneTag::ALL {
        assert_eq!(SceneTag::from_tag(tag.name()), Some(tag));
    }
    assert_eq!(SceneTag::from_tag("div"), None);
    assert_eq!(SceneTag::from_tag("Mesh"), None);
}

#[test]
fn a_longhand_a_tag_does_not_read_names_what_to_use_instead() {
    // Given
    let width = PropertyId::named("width");
    let opacity = PropertyId::named("opacity");

    // Then
    assert!(instead(width).contains("<Mesh geometry>"));
    assert!(instead(opacity).contains("each `mesh`") && instead(opacity).contains("D3"));
}
