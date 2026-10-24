<style>
/* viewport 640x400 390x800 */
/* scroll containers (F2c S9): each scrolled with data-scroll, which
   the oracle page and the corpus test both apply; Chromium clamps it to
   the content as vapors does. A list scrolled part way, one past its
   end (clamped), a padded and bordered one, one past its right edge, a flex column, a nested
   pair, and absolute children placed inside and outside the container.
   The oracle hides scrollbars, so they take no space here, as vapors'
   overlay scrollbars take none. */
body { margin: 8px; font-size: 14px; }
.list { width: 200px; height: 100px; overflow: auto; background-color: #eef; }
.row { height: 30px; margin-bottom: 4px; background-color: #ccd; }
.padded { padding: 10px; border: 3px solid #333; }
.column { display: flex; flex-direction: column; gap: 6px; }
.outer { width: 200px; height: 120px; overflow: scroll; }
.inner { height: 60px; overflow: auto; margin-bottom: 20px; }
.placed { position: relative; }
.wide { width: 300px; margin-right: 10px; }
.abs { position: absolute; top: 40px; left: 20px; width: 30px; height: 20px; background-color: #f80; }
</style>

<div class="list" data-scroll="0 25"><div class="row"></div><div class="row"></div><div class="row"></div><div class="row"></div><div class="row"></div></div>
<div class="list" data-scroll="0 9999"><div class="row"></div><div class="row"></div><div class="row"></div><div class="row"></div></div>
<div class="list padded" data-scroll="0 9999"><div class="row"></div><div class="row"></div><div class="row"></div><div class="row"></div></div>
<div class="list padded" data-scroll="9999 0"><div class="row wide"></div><div class="row"></div></div>
<div class="list column" data-scroll="0 50"><div class="row"></div><div class="row"></div><div class="row"></div><div class="row"></div></div>
<div class="outer" data-scroll="0 30"><div class="inner" data-scroll="0 20"><div class="row"></div><div class="row"></div><div class="row"></div></div><div class="row"></div><div class="row"></div><div class="row"></div></div>
<div class="list placed" data-scroll="0 40"><div class="row"></div><div class="row"></div><div class="row"></div><div class="row"></div><div class="abs"></div></div>
<div class="list" data-scroll="0 40"><div class="row"></div><div class="row"></div><div class="row"></div><div class="row"></div><div class="abs"></div></div>
