<style>
/* viewport 640x400 */
/* paint */
/* A framed lane (docs/plans/splice-layout.md, section 11), drawn by
   vapors and by Chromium: a list of 40 rows of a whole 12 px, so each
   row is a lane frame of its own at 1x and 2x, below a header and
   inside a padded, bordered list, so each frame's shift holds the
   list's origin; the rows past the view are culled through the lane.
   Boxes only, no text, so the comparison is of the frames' shifts.
   The corpus asserts the list is framed (gpu_paint.rs). */
body { margin: 0; padding: 16px; background-color: #ffffff; }
.head { height: 24px; background-color: #1d1d1f; }
.list { padding: 4px; border: 2px solid #555555; }
.row { height: 12px; padding-left: 4px; }
.mark { width: 20px; height: 6px; margin-top: 3px; background-color: #1f6feb; }
.r0 { background-color: #dde3ea; }
.r1 { background-color: #f5b041; }
.r2 { background-color: #ffffff; }
</style>

<div class="head"></div>
<div class="list">
<div class="row r0"><div class="mark"></div></div>
<div class="row r1"><div class="mark"></div></div>
<div class="row r2"><div class="mark"></div></div>
<div class="row r0"><div class="mark"></div></div>
<div class="row r1"><div class="mark"></div></div>
<div class="row r2"><div class="mark"></div></div>
<div class="row r0"><div class="mark"></div></div>
<div class="row r1"><div class="mark"></div></div>
<div class="row r2"><div class="mark"></div></div>
<div class="row r0"><div class="mark"></div></div>
<div class="row r1"><div class="mark"></div></div>
<div class="row r2"><div class="mark"></div></div>
<div class="row r0"><div class="mark"></div></div>
<div class="row r1"><div class="mark"></div></div>
<div class="row r2"><div class="mark"></div></div>
<div class="row r0"><div class="mark"></div></div>
<div class="row r1"><div class="mark"></div></div>
<div class="row r2"><div class="mark"></div></div>
<div class="row r0"><div class="mark"></div></div>
<div class="row r1"><div class="mark"></div></div>
<div class="row r2"><div class="mark"></div></div>
<div class="row r0"><div class="mark"></div></div>
<div class="row r1"><div class="mark"></div></div>
<div class="row r2"><div class="mark"></div></div>
<div class="row r0"><div class="mark"></div></div>
<div class="row r1"><div class="mark"></div></div>
<div class="row r2"><div class="mark"></div></div>
<div class="row r0"><div class="mark"></div></div>
<div class="row r1"><div class="mark"></div></div>
<div class="row r2"><div class="mark"></div></div>
<div class="row r0"><div class="mark"></div></div>
<div class="row r1"><div class="mark"></div></div>
<div class="row r2"><div class="mark"></div></div>
<div class="row r0"><div class="mark"></div></div>
<div class="row r1"><div class="mark"></div></div>
<div class="row r2"><div class="mark"></div></div>
<div class="row r0"><div class="mark"></div></div>
<div class="row r1"><div class="mark"></div></div>
<div class="row r2"><div class="mark"></div></div>
<div class="row r0"><div class="mark"></div></div>
</div>
