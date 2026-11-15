//! The view uniform's layout in each shader that reads it agrees with
//! the bytes `instanced.rs` writes: the table starts where the head
//! ends, and holds `RowOffsets::CAPACITY` rows.

use super::VIEW_SIZE;
use crate::RowOffsets;

/// The offset and length of struct member `rows` of global `name` in the
/// shader `source`, as naga lays the WGSL out.
fn rows_layout(source: &str, name: &str) -> (u32, u32) {
    let module = naga::front::wgsl::parse_str(&RowOffsets::shader(source)).expect("parses");
    let mut global = module.global_variables.iter().map(|(_, global)| global);
    let view = global.find(|global| global.name.as_deref() == Some(name));
    let ty = &module.types[view.expect("the uniform").ty];
    let naga::TypeInner::Struct { members, .. } = &ty.inner else {
        panic!("{name} is not a struct");
    };
    let rows = members
        .iter()
        .find(|member| member.name.as_deref() == Some("rows"));
    let rows = rows.expect("a rows member");
    let naga::TypeInner::Array { size, .. } = module.types[rows.ty].inner else {
        panic!("rows is not an array");
    };
    let naga::ArraySize::Constant(len) = size else {
        panic!("rows has no constant length");
    };
    (rows.offset, len.get())
}

#[test]
fn each_instanced_shaders_table_starts_after_the_head_and_holds_the_capacity() {
    let head = u32::try_from(VIEW_SIZE - RowOffsets::CAPACITY * 8).expect("small");
    let pairs = u32::try_from(RowOffsets::CAPACITY / 2).expect("small");
    for (name, source) in [
        ("quads", include_str!("quads.wgsl")),
        ("shadows", include_str!("shadows.wgsl")),
    ] {
        assert_eq!(rows_layout(source, "view"), (head, pairs), "{name}.wgsl");
    }
}
