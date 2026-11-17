//! A material checked against the contract when it is given.

use crate::{Material, MaterialError, Meshes};

fn given(wgsl: &str) -> Result<crate::MaterialId, MaterialError> {
    Meshes::new().material(Material {
        label: "ground",
        wgsl: wgsl.into(),
    })
}

#[test]
fn a_material_reading_only_the_contract_is_held() {
    let ok = given(
        "const TINT: f32 = 0.5;\nfn material(at: MaterialInput) -> vec4<f32> { return at.colour * TINT; }",
    );
    assert!(ok.is_ok(), "{ok:?}");
}

#[test]
fn a_syntax_error_is_refused_naming_the_label_and_the_line_in_its_own_source() {
    let refused = given("fn helper() -> f32 { return 1.0; }\nfn material(at: MaterialInput) -> vec4<f32> { return at.colour }")
        .expect_err("refused");
    assert!(
        matches!(refused, MaterialError::Wgsl { line: Some(2), .. }),
        "{refused:?}"
    );
    assert!(
        refused
            .to_string()
            .starts_with("material `ground`, line 2: "),
        "{refused}"
    );
}

#[test]
fn reading_the_scene_renderers_camera_is_refused() {
    let refused = given(
        "fn material(at: MaterialInput) -> vec4<f32> {\n  return camera.view_projection[0];\n}",
    )
    .expect_err("refused");
    assert!(
        matches!(refused, MaterialError::Wgsl { line: Some(2), .. }),
        "{refused:?}"
    );
    assert!(refused.to_string().contains("camera"), "{refused}");
}

#[test]
fn reading_the_scene_renderers_node_is_refused() {
    let refused = given("fn material(at: MaterialInput) -> vec4<f32> { return node.colour; }")
        .expect_err("refused");
    assert!(
        matches!(refused, MaterialError::Wgsl { line: Some(1), .. }),
        "{refused:?}"
    );
}

#[test]
fn a_binding_of_its_own_is_refused_naming_it() {
    let refused = given("@group(2) @binding(0) var<uniform> tint: vec4<f32>;\nfn material(at: MaterialInput) -> vec4<f32> { return tint; }")
        .expect_err("refused");
    assert_eq!(
        refused,
        MaterialError::Binding {
            label: "ground",
            name: "tint".into()
        }
    );
}

#[test]
fn a_name_the_renderer_uses_is_refused_naming_it() {
    let refused = given("fn on_plane() -> f32 { return 1.0; }\nfn material(at: MaterialInput) -> vec4<f32> { return at.colour; }")
        .expect_err("refused");
    assert_eq!(
        refused,
        MaterialError::Reserved {
            label: "ground",
            name: "on_plane".into()
        }
    );
}

#[test]
fn a_material_with_no_material_function_or_another_shape_is_refused() {
    let missing = given("fn shade(at: MaterialInput) -> vec4<f32> { return at.colour; }");
    let other = given("fn material(at: MaterialInput) -> vec3<f32> { return at.colour.rgb; }");
    let signature = MaterialError::Signature { label: "ground" };
    assert_eq!((missing, other), (Err(signature.clone()), Err(signature)));
}

#[test]
fn an_entry_point_named_as_the_renderers_is_refused_naming_it() {
    let refused = given("fn material(at: MaterialInput) -> vec4<f32> { return at.colour; }\n@fragment fn fs_material() -> @location(0) vec4<f32> { return vec4(1.0); }")
        .expect_err("refused");
    assert_eq!(
        refused,
        MaterialError::Reserved {
            label: "ground",
            name: "fs_material".into()
        }
    );
}

#[test]
fn a_material_taking_derivatives_of_its_input_is_held() {
    let ok = given(
        "fn material(at: MaterialInput) -> vec4<f32> {\n    let ramp = fwidth(at.world.xz) + abs(dpdx(at.world.xz)) + abs(dpdy(at.world.xz));\n    return at.colour * min(ramp.x + ramp.y, 1.0);\n}",
    );
    assert!(ok.is_ok(), "{ok:?}");
}
