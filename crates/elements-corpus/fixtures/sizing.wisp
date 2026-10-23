<style>
/* viewport 640x400 */
body { margin: 0; }
.row { display: flex; gap: 10px; }
.min { width: min-content; background-color: #eee; }
.max { width: max-content; background-color: #ddd; }
.fit { width: fit-content; background-color: #ccc; }
.calc { width: calc(50% - 20px); height: calc(10px + 5%); background-color: #c33; margin-top: 8px; }
.pad { padding-left: calc(10% + 4px); height: 10px; background-color: #3c3; }
.basis { display: flex; margin-top: 8px; }
.basis div { flex-basis: content; height: 12px; background-color: #33c; }
</style>

<div class="row"><div class="min">two words</div><div class="max">some more words here</div></div>
<div class="fit">fits its content</div>
<div class="calc"></div>
<div class="pad"></div>
<div class="basis"><div>a b</div><div>c d e</div></div>
