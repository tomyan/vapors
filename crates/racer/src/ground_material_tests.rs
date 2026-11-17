//! The ground as a material: it reads only the contract, and its
//! derivatives are taken before `ground()`, whose region guards branch
//! on the fragment's position (G1). naga accepts a derivative in
//! non-uniform control flow (checked 2026-11-17), so these tests stand
//! in for its uniformity check.

use naga::{Expression, Function, Statement};

use super::{MATERIAL, ground_material, ground_wgsl};

fn module(source: &str) -> naga::Module {
    naga::front::wgsl::parse_str(source).expect("the ground's WGSL parses")
}

fn takes_a_derivative(function: &Function) -> bool {
    let derivative = |(_, e): (_, &Expression)| matches!(e, Expression::Derivative { .. });
    function.expressions.iter().any(derivative)
}

#[test]
fn the_ground_reads_only_the_material_contract() {
    let held = vapors_scene3d::Meshes::new().material(ground_material());
    assert!(held.is_ok(), "{held:?}");
}

#[test]
fn no_function_of_the_ground_takes_a_derivative_so_its_guards_may_branch() {
    // Given
    let module = module(&ground_wgsl());

    // When
    let deriving: Vec<_> = module
        .functions
        .iter()
        .filter(|(_, function)| takes_a_derivative(function))
        .map(|(_, function)| function.name.clone())
        .collect();

    // Then
    assert!(
        deriving.is_empty(),
        "derivatives in {deriving:?}: take them in material()"
    );
}

#[test]
fn the_material_takes_its_derivatives_at_its_top_level_before_calling_the_ground() {
    // Given
    let source = [vapors_scene3d::MATERIAL_WGSL, &ground_wgsl(), MATERIAL].concat();
    let module = module(&source);
    let (_, material) = module
        .functions
        .iter()
        .find(|(_, f)| f.name.as_deref() == Some("material"))
        .expect("a material function");

    // When: the top-level statements up to the call of ground()
    let ground = module
        .functions
        .iter()
        .find(|(_, f)| f.name.as_deref() == Some("ground"))
        .map(|(handle, _)| handle);
    let before_ground = material.body.iter().take_while(
        |s| !matches!(s, Statement::Call { function, .. } if Some(*function) == ground),
    );
    let emitted_before: Vec<_> = before_ground
        .filter_map(|s| match s {
            Statement::Emit(range) => Some(range.clone()),
            _ => None,
        })
        .flatten()
        .collect();

    // Then: every derivative is among them
    assert!(takes_a_derivative(material));
    for (handle, expression) in material.expressions.iter() {
        if matches!(expression, Expression::Derivative { .. }) {
            assert!(
                emitted_before.contains(&handle),
                "{expression:?} is taken after or inside a branch"
            );
        }
    }
}
