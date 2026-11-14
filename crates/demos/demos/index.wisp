<script>
let demos: Signal<Vec<&'static crate::Listed>> = Signal::new(crate::DEMOS.iter().collect());
</script>

<style>
:global(body) { margin: 0; padding: 24px; background-color: #171b1f; color: #e4e8ec; font-size: 15px; }
h1 { margin-top: 0; font-size: 20px; }
.demo { display: flex; margin-top: 8px; color: #e4e8ec; }
.name { width: 140px; flex-shrink: 0; color: #8ab4f8; }
</style>

<h1>vapors demos</h1>
<p>Each opens its demo; the page address names it with ?demo=.</p>
{for demo in demos.get() key=demo.name}<a class="demo" href={demo.get().href()}><span class="name">{demo.get().title}</span><span>{demo.get().line}</span></a>{/for}
