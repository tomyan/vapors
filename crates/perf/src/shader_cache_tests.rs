//! The shader cache's state, from the environment and the directory.

use super::{ShaderCache, shader_cache};

fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
    move |name| {
        pairs
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| (*value).to_owned())
    }
}

#[test]
fn an_unset_cache_is_refused_naming_both_variables() {
    let error = shader_cache(env(&[])).expect_err("uncontrolled");
    assert!(error.contains("MESA_SHADER_CACHE_DIR"), "{error}");
    assert!(error.contains("MESA_SHADER_CACHE_DISABLE=true"), "{error}");
}

#[test]
fn a_disabled_cache_is_said_to_be() {
    let cache = shader_cache(env(&[("MESA_SHADER_CACHE_DISABLE", "true")]));
    assert_eq!(cache, Ok(ShaderCache::Disabled));
}

#[test]
fn an_empty_directory_is_cold_and_a_filled_one_warm() {
    // Given
    let dir = tempfile::tempdir().expect("a directory");
    let path = dir.path().to_str().expect("utf-8").to_owned();
    let named = [("MESA_SHADER_CACHE_DIR", path.as_str())];

    // When
    let cold = shader_cache(env(&named)).expect("readable").describe();
    std::fs::create_dir(dir.path().join("mesa_shader_cache")).expect("an entry");
    let warm = shader_cache(env(&named)).expect("readable").describe();

    // Then
    assert!(cold.ends_with("empty at the start (cold)"), "{cold}");
    assert!(warm.ends_with("1 entries at the start (warm)"), "{warm}");
}

#[test]
fn a_missing_directory_is_refused_naming_it() {
    let error =
        shader_cache(env(&[("MESA_SHADER_CACHE_DIR", "/nonexistent/cache")])).expect_err("missing");
    assert!(
        error.contains("/nonexistent/cache cannot be read"),
        "{error}"
    );
}
