//! The params uniform's layout in `text.wgsl` agrees with the bytes
//! `draw.rs` writes: the table starts where the head ends, and holds
//! `RowOffsets::CAPACITY` rows.

use vapors_renderer::RowOffsets;

use super::PARAMS_HEAD;

#[test]
fn the_text_shaders_table_starts_after_the_head_and_holds_the_capacity() {
    // Given
    let source = RowOffsets::shader(include_str!("text.wgsl"));
    let module = naga::front::wgsl::parse_str(&source).expect("parses");

    // When
    let mut global = module.global_variables.iter().map(|(_, global)| global);
    let params = global.find(|global| global.name.as_deref() == Some("params"));
    let naga::TypeInner::Struct { members, .. } = &module.types[params.expect("params").ty].inner
    else {
        panic!("params is not a struct");
    };
    let rows = members
        .iter()
        .find(|member| member.name.as_deref() == Some("rows"));
    let rows = rows.expect("a rows member");
    let naga::TypeInner::Array {
        size: naga::ArraySize::Constant(len),
        ..
    } = module.types[rows.ty].inner
    else {
        panic!("rows has no constant length");
    };

    // Then
    let pairs = u32::try_from(RowOffsets::CAPACITY / 2).expect("small");
    let head = u32::try_from(PARAMS_HEAD).expect("small");
    assert_eq!((rows.offset, len.get()), (head, pairs));
}
