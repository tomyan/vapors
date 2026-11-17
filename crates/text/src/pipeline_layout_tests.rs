//! The params uniform's layout in `text.wgsl` agrees with the bytes
//! `draw.rs` writes (`params_bytes`): gamma, contrast, the viewport, then
//! the draw's first row entry at 16, within `PARAMS_SIZE`; the transform
//! table is the storage buffer at binding 3, the clip table at 4.

use super::{PARAMS_SIZE, text_source};

#[test]
fn the_text_shader_reads_the_params_and_the_table_as_written() {
    // Given
    let module = naga::front::wgsl::parse_str(&text_source()).expect("parses");
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
    let binding = |name| {
        named(name)
            .and_then(|t| t.binding.as_ref())
            .map(|b| b.binding)
    };
    let (table, clips) = (binding("transforms"), binding("clips"));

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
    assert_eq!((table, clips), (Some(3), Some(4)));
}
