<style>
/* viewport 390x800 768x1024 1280x800 */
/* touche's shell: a 44 px bar over the canvas; a side panel beside it
   from 600 px wide, and under 600 px the canvas alone. */
html, body { height: 100%; }
body { margin: 0; display: grid; grid-template-rows: 44px minmax(0, 1fr); grid-template-columns: minmax(0, 1fr) 280px; background-color: #e9ecef; }
.bar { grid-column-start: 1; grid-column-end: 3; display: flex; align-items: center; gap: 8px; padding: 0 8px; background-color: #171b1f; }
.bar .title { flex-grow: 1; min-width: 0; height: 20px; background-color: #333; }
.bar .action { width: 72px; height: 32px; background-color: #3a7bd5; border-radius: 6px; }
.canvas { background-color: #2b2b2b; }
.panel { border-left: 1px solid #c7ccd1; background-color: #ffffff; padding: 12px; display: flex; flex-direction: column; gap: 8px; }
.panel .row { height: 28px; background-color: #f1f3f5; }
@media (width < 600px) {
  body { grid-template-columns: minmax(0, 1fr); }
  .bar { grid-column-end: 2; }
  .panel { display: none; }
}
</style>

<div class="bar"><div class="title"></div><div class="action"></div><div class="action"></div></div>
<div class="canvas"></div>
<div class="panel"><div class="row"></div><div class="row"></div><div class="row"></div></div>
