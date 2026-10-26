<script>
use super::scope_component::{ScopeComponent, ScopeComponentProps};

let name: String = String::from("Svelte");
</script>

{for character in name.chars().collect::<Vec<char>>()}<p>{character}</p><ScopeComponent>{snippet inner()}<a href="#">{character}</a>{/snippet}</ScopeComponent>{/for}
