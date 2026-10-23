<style>
html, body { height: 100%; }
body {
  margin: 0;
  display: grid;
  grid-template-rows: 44px minmax(0, 1fr);
  grid-template-columns: minmax(0, 1fr) 280px;
  background-color: #e9ecef;
  font-size: 14px;
}
.bar {
  grid-column-start: 1;
  grid-column-end: 3;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 8px;
  background-color: #171b1f;
  color: #e4e8ec;
}
.title { flex-grow: 1; min-width: 0; }
button { cursor: pointer; padding: 6px 12px; border: none; border-radius: 6px; background-color: #3a7bd5; color: #ffffff; }
.canvas { background-color: #2b2b2b; }
.panel {
  border-left: 1px solid #c7ccd1;
  background-color: #ffffff;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.layer { height: 28px; padding: 0 8px; display: flex; align-items: center; background-color: #f1f3f5; border-radius: 4px; }
@media (width < 600px) {
  body { grid-template-columns: minmax(0, 1fr); }
  .bar { grid-column-end: 2; }
  .panel { display: none; }
}
</style>

<div class="bar"><div class="title">touche</div><button>Open</button><button>Export</button></div>
<div class="canvas"></div>
<div class="panel">
  <div class="layer">Curves</div>
  <div class="layer">White point</div>
  <div class="layer">Background</div>
</div>
