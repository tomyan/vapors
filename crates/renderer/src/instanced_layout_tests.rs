//! The view uniform's layout in each shader that reads the transform
//! table agrees with the bytes `instanced.rs` writes (`view_bytes`): the
//! size at 0, the rows' first entry at 8, within `VIEW_SIZE` bytes; and
//! the table is the storage buffer at binding 1.

use super::{VIEW_SIZE, instanced_source};

/// The view's members as (name, offset), its span, and the table's
/// binding, in the shader `source` as naga lays the WGSL out.
fn layout(source: &str) -> (Vec<(String, u32)>, u32, Option<u32>) {
    let module = naga::front::wgsl::parse_str(source).expect("parses");
    let named = |name: &str| {
        module
            .global_variables
            .iter()
            .map(|(_, global)| global)
            .find(|global| global.name.as_deref() == Some(name))
    };
    let ty = &module.types[named("view").expect("the uniform").ty];
    let naga::TypeInner::Struct { members, span } = &ty.inner else {
        panic!("view is not a struct");
    };
    let members = members
        .iter()
        .map(|m| (m.name.clone().unwrap_or_default(), m.offset))
        .collect();
    let binding = named("transforms")
        .and_then(|t| t.binding.as_ref())
        .map(|b| b.binding);
    (members, *span, binding)
}

#[test]
fn each_instanced_shader_reads_the_view_and_the_table_as_written() {
    for (name, source) in [
        ("quads", instanced_source(include_str!("quads.wgsl"))),
        ("shadows", instanced_source(include_str!("shadows.wgsl"))),
        (
            "closed forms",
            instanced_source(concat!(
                include_str!("closed_forms_2d.wgsl"),
                include_str!("closed_forms_shared.wgsl")
            )),
        ),
    ] {
        let (members, span, binding) = layout(&source);
        let expected = [("size".to_owned(), 0), ("rows_base".to_owned(), 8)];
        assert_eq!(members, expected, "{name}");
        assert!(span as usize <= VIEW_SIZE, "{name}: a view of {span} bytes");
        assert_eq!(binding, Some(1), "{name}: the table's binding");
    }
}
