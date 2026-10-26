<script>
use crate::card::{Card, CardProps};

#[prop]
let children: Snippet = Snippet::empty();
</script>

{if true}<Card title="a">{render children()}</Card>{/if}{render children()}
