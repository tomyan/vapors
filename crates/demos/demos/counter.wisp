<script>
let count: Signal<i32> = Signal::new(0);
let increment = move || count.update(|n| *n += 1);
</script>

<style>
:global(html), :global(body) { height: 100%; }
:global(body) {
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
<div class="strip" class:waiting={count.get() == 0} class:counted={count.get() > 0}></div>
