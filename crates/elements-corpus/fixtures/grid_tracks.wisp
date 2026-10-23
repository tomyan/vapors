<style>
/* viewport 640x400 */
body { margin: 8px; }
.grid { display: grid; grid-template-columns: 100px 1fr 2fr 20%; grid-auto-rows: 30px; gap: 4px 6px; background-color: #eee; }
.grid div { background-color: #c33; }
.wide { grid-column-start: span 2; }
.tall { grid-row-start: 2; grid-row-end: 4; grid-column-start: 4; }
.minmax { display: grid; grid-template-columns: minmax(50px, 100px) repeat(3, 1fr) minmax(0, 1fr); margin-top: 10px; height: 40px; }
.minmax div { background-color: #3c3; }
.flow { display: grid; grid-template-rows: 20px 20px; grid-auto-flow: column; grid-auto-columns: 40px; margin-top: 10px; justify-items: center; }
.flow div { width: 20px; height: 10px; background-color: #33c; }
</style>

<div class="grid"><div></div><div class="wide"></div><div></div><div class="tall"></div><div></div><div></div><div></div></div>
<div class="minmax"><div></div><div></div><div></div><div></div><div></div></div>
<div class="flow"><div></div><div></div><div></div><div></div><div></div></div>
