<script>
let outer: Signal<bool> = Signal::new(true);
let inner: Signal<bool> = Signal::new(true);
</script>

<div>{if outer.get()}{if inner.get()}<p>inner</p>{/if}{else}<span>else</span>{/if}</div>
