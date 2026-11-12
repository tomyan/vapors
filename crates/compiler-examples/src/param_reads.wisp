<script>
#[prop]
let word: Signal<String> = Signal::new(String::new());
</script>

{render twice(word)}
{snippet twice(name: String)}<b>{name}</b><i>{name}</i>{if name.get().len() > 3}<u>{name}</u>{/if}{render quoted(name.clone())}{/snippet}
{snippet quoted(said: String)}<s>{said}</s>{/snippet}
