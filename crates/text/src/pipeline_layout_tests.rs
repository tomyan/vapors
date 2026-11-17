//! The params uniform's layout in `text.wgsl` agrees with the bytes
//! `draw.rs` writes (`params_bytes`): gamma, contrast, the viewport, then
//! the draw's first row entry at 16, within `PARAMS_SIZE`; and the
//! transform table is the storage buffer at binding 3.

use super::PARAMS_SIZE;

#[test]
fn the_text_shader_reads_the_params_and_the_table_as_written() {
    // Given
    let module = naga::front::wgsl::parse_str(include_str!("text.wgsl")).expect("parses");
    let named = |name: &str| {
        module
            .global_variables
            .iter()
            .map(|(_, global)| global)
            .find(|global| global.name.as_deref() == Some(name))
    };

    // When
    let params = &module.types[named("params").expect("params").ty].inner;
    let naga::TypeInner::Struct { members, span } = params else {
        panic!("params is not a struct");
    };
    let offsets: Vec<_> = members
        .iter()
        .map(|m| (m.name.clone().unwrap_or_default(), m.offset))
        .collect();
    let table = named("transforms")
        .and_then(|t| t.binding.as_ref())
        .map(|b| b.binding);

    // Then
    let expected = [
        ("gamma", 0),
        ("contrast", 4),
        ("viewport", 8),
        ("rows_base", 16),
    ]
    .map(|(name, at)| (name.to_owned(), at));
    assert_eq!(offsets, expected);
    assert!(*span as usize <= PARAMS_SIZE, "params of {span} bytes");
    assert_eq!(table, Some(3));
}
