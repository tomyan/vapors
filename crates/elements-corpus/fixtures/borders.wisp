<style>
/* viewport 640x400 */
body { margin: 10px; }
.card { border: 3px solid #333; padding: 8px; margin-bottom: 6px; border-radius: 6px; box-shadow: 0 2px 6px #0004; }
.thin { border-top: thin solid red; border-bottom: thick solid blue; height: 10px; }
.mixed { border-left: 0.5px solid black; border-right: 2.7px solid black; width: 100px; height: 4px; }
.row { display: flex; gap: 4px; }
.row div { border: medium solid green; width: 20px; height: 20px; box-sizing: border-box; }
hr { margin: 4px 0; }
.boxed { border: 1px solid #999; padding: 4px; font-size: 13px; }
</style>

<div class="card"><div class="thin"></div></div>
<div class="mixed"></div>
<hr />
<div class="row"><div></div><div></div><div></div></div>
<div class="boxed">A bordered box with text inside it.</div>
