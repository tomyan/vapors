<script>
let count: Signal<i32> = Signal::new(0);
let increment = move || count.update(|n| *n += 1);
let strip: Memo<String> = Memo::new(move || {
    let state = if count.get() > 0 { "counted" } else { "waiting" };
    format!("strip {state}")
});
</script>

<style>
html, body { height: 100%; }
body {
  margin: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  background-color: #e9ecef;
}
button {
  padding: 12px 20px;
  font-size: 15px;
  background-color: #171b1f;
  color: #e4e8ec;
  border: none;
  border-radius: 6px;
  cursor: pointer;
}
button:hover { background-color: #2a3036; }
button:active { background-color: #3a7bd5; }
.strip { margin-top: 16px; width: 240px; height: 8px; }
.waiting { background-color: #c7ccd1; }
.counted { background-color: #2e7d32; }
</style>

<button onclick={increment}>Clicked {count} times</button>
<div class={strip.get()}></div>
