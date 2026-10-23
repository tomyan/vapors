<script>
let taps: Signal<i32> = Signal::new(0);
let tap = move || taps.update(|n| *n += 1);
let word: Memo<String> = Memo::new(move || {
    let state = if taps.get() > 0 { "tapped" } else { "waiting" };
    format!("tap {state}")
});
</script>

<style>
/* Inline text (F2c S6): one paragraph of mixed inline elements, which
   wraps; a code background behind its text; and a span that counts its
   own taps and turns yellow once tapped. */
:global(html), :global(body) { height: 100%; }
:global(body) {
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: #e9ecef;
  font-size: 16px;
}
.page {
  width: 340px;
  padding: 20px;
  background-color: #ffffff;
  border: 1px solid #c7ccd1;
  border-radius: 8px;
}
h2 { margin: 0 0 8px 0; font-size: 22px; color: #171b1f; }
p { margin: 0; line-height: 24px; color: #171b1f; }
code { font-family: "Geist Mono", monospace; background-color: #dbe4ff; }
.tap { color: #1f5fbf; cursor: pointer; }
.tapped { background-color: #ffe066; }
</style>

<div class="page">
  <h2>Inline text</h2>
  <p>A paragraph with <strong>strong</strong>, <em>emphasis</em> and <code>code</code> in it, wrapping over the end of its line. <span class={word.get()} onclick={tap}>Tap this</span>, tapped {taps} times.</p>
</div>
