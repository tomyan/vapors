<script>
#[prop]
let label: String = String::from("kept");
#[prop]
let shown: Signal<bool> = Signal::new(true);
</script>

{snippet tag(text: String)}<b>{text}</b>{/snippet}
{if shown.get()}{render tag(label.clone())}{/if}
<p>{label}</p>
