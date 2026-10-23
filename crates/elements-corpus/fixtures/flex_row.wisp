<style>
/* viewport 640x400 */
body { margin: 8px; }
.row { display: flex; padding: 4px; background-color: #ddd; }
.a { width: 50px; height: 30px; background-color: #c33; }
.b { flex-grow: 1; height: 20px; margin: 0 10px; background-color: #3c3; }
.c { flex-basis: 25%; height: 40px; background-color: #33c; }
.column { display: flex; flex-direction: column; align-items: center; margin-top: 10px; }
.d { width: 60%; height: 10px; background-color: #999; }
.e { width: 100px; height: 12px; margin-top: auto; background-color: #555; }
</style>

<div class="row"><div class="a"></div><div class="b"></div><div class="c"></div></div>
<div class="column"><div class="d"></div><div class="e"></div></div>
