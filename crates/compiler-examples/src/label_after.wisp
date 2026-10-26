<script>
use crate::card::{Card, CardProps};

#[prop]
let label: String = String::new();
let shown: Signal<bool> = Signal::new(true);
</script>

{if shown.get()}<Card title="a"><p>{label}</p></Card>{/if}<p>{label}</p>
