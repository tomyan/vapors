<script>
#[prop]
let names: Signal<Vec<String>> = Signal::new(Vec::new());
let first: Signal<Option<NodeId>> = Signal::new(None);
let shown: Signal<bool> = Signal::new(true);
let row: Signal<Option<NodeId>> = Signal::new(None);
</script>

<div>{if shown.get()}<p bind:this={first}>first</p>{/if}{for name in names.get() key=name.clone()}<i bind:this={row}>{name}</i>{/for}</div>
