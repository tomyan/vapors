<script>
use crate::card::{Card, CardProps};

#[prop]
let children: Snippet = Snippet::empty();
</script>

{render children()}<i>after</i>{if true}<Card title="inner">{render children()}</Card>{/if}
