<script>
let pressed: Signal<i32> = Signal::new(0);
let press = move || pressed.update(|n| *n += 1);
</script>

<style>
html, body { height: 100%; }
body {
  margin: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 28px;
  background-color: #e9ecef;
  font-size: 14px;
}
.cards { display: flex; gap: 24px; }
.card {
  width: 150px;
  height: 90px;
  padding: 12px;
  background-color: #ffffff;
  border-radius: 10px;
  box-shadow: 0 6px 16px #00000040;
}
.sides {
  width: 150px;
  height: 90px;
  padding: 12px;
  background-color: #ffffff;
  border-top: 3px solid #d32f2f;
  border-right: 3px solid #388e3c;
  border-bottom: 3px solid #1976d2;
  border-left: 3px solid #fbc02d;
}
.outline {
  width: 150px;
  height: 90px;
  padding: 12px;
  border: 2px solid #171b1f;
  border-radius: 16px;
}
.buttons { display: flex; gap: 12px; }
button { cursor: pointer; font-size: 14px; padding: 8px 16px; }
.primary { background-color: #3a7bd5; color: #ffffff; border: 1px solid #2f63ad; border-radius: 6px; }
.primary:hover { background-color: #2f6cc2; }
.primary:active { background-color: #24569e; }
.quiet { background-color: transparent; border: 1px solid #8f8f9d; border-radius: 6px; }
.quiet:hover { background-color: #dde1e5; }
.pill { background-color: #2e7d32; color: #ffffff; border: none; border-radius: 9999px; }
.pill:hover { background-color: #276a2b; }
</style>

<div class="cards">
  <div class="card">A card: rounded corners and a soft shadow.</div>
  <div class="sides">Four borders, one colour per side.</div>
  <div class="outline">An outline with round corners.</div>
</div>
<div class="buttons">
  <button class="primary" onclick={press}>Primary</button>
  <button class="quiet" onclick={press}>Quiet</button>
  <button class="pill" onclick={press}>Pressed {pressed}</button>
</div>
