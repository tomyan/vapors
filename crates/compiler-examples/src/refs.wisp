<script>
#[prop]
let names: Signal<Vec<String>> = Signal::new(Vec::new());
#[prop]
let top: Signal<Option<NodeId>> = Signal::new(None);
let first: Signal<Option<NodeId>> = Signal::new(None);
let shown: Signal<bool> = Signal::new(true);
let rows: Signal<std::collections::BTreeMap<String, NodeId>> = Signal::new(std::collections::BTreeMap::new());
let by_index: Signal<std::collections::BTreeMap<usize, NodeId>> = Signal::new(std::collections::BTreeMap::new());
</script>

<div bind:this={top}>{if shown.get()}<p bind:this={first}>first</p>{/if}{for name in names.get() key=name.clone()}<i bind:this={rows}>{name}</i>{/for}{for (i, name) in names.get() key=i}<b bind:this={by_index}>{name}{i}</b>{/for}</div>
