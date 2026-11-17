use super::*;

#[test]
fn the_vertex_attributes_fill_exactly_the_vertex_size_the_scene_checks() {
    // Given
    let last = VERTEX_ATTRIBUTES.last().expect("attributes");

    // When
    let filled = last.offset + last.format.size();

    // Then
    assert_eq!(
        filled, VERTEX_BYTES as u64,
        "the pipeline's vertex attributes end at {filled} bytes, the scene's VERTEX_BYTES is {VERTEX_BYTES}: change both together (vapors-scene3d mesh_error.rs and pipeline.rs)"
    );
}

#[test]
fn the_names_a_material_may_not_define_are_those_the_renderer_puts_beside_it() {
    // Given: the renderer's side of a material's module, around the
    // contract and a material defining nothing else
    let material = "fn material(at: MaterialInput) -> vec4<f32> { return at.colour; }";
    let source = [
        PROJECTIVE_WGSL,
        MATERIAL_WGSL,
        material,
        include_str!("material.wgsl"),
    ]
    .concat();
    let module = naga::front::wgsl::parse_str(&source).expect("parses");

    // When: every name it defines at the top, but the contract's
    let functions = module.functions.iter().filter_map(|(_, f)| f.name.clone());
    let entries = module.entry_points.iter().map(|e| e.name.clone());
    let types = module.types.iter().filter_map(|(_, t)| t.name.clone());
    let constants = module.constants.iter().filter_map(|(_, c)| c.name.clone());
    let globals = module
        .global_variables
        .iter()
        .filter_map(|(_, g)| g.name.clone());
    let mut names: Vec<String> = functions
        .chain(entries)
        .chain(types)
        .chain(constants)
        .chain(globals)
        .collect();
    names.retain(|name| name != "material" && name != "MaterialInput");
    names.sort();
    names.dedup();

    // Then
    let reserved: Vec<String> = vapors_scene3d::RESERVED_NAMES
        .iter()
        .map(|&n| n.to_owned())
        .collect();
    assert_eq!(
        names, reserved,
        "change vapors-scene3d material_check.rs RESERVED_NAMES with the renderer's WGSL"
    );
}
