<style>
/* viewport 640x400 */
/* paint: compared with Chromium's pixels in tests/gpu_paint.rs */
/* stacking (F2c S8): overlapping squares painted by z-index against
   tree order, a negative z-index under an in-flow block of the same
   context (`.scene` roots it with z-index 0), a z-index
   auto box whose positioned child joins the outer stacking order, and
   a z-index context whose children cannot rise above a later sibling. */
body { margin: 0; padding: 16px; background-color: #e9ecef; }
.scene { position: relative; z-index: 0; height: 160px; }
.sq { position: absolute; width: 60px; height: 60px; }
.a { left: 0; top: 0; z-index: 3; background-color: #d32f2f; }
.b { left: 30px; top: 30px; z-index: 1; background-color: #1976d2; }
.c { left: 45px; top: 10px; z-index: 2; background-color: #2e7d32; }
.under { left: 160px; top: 0; z-index: -1; background-color: #fbc02d; }
.block { margin-left: 140px; width: 100px; height: 40px; background-color: #171b1f; }
.auto { left: 280px; top: 0; background-color: #3a7bd5; }
.escape { left: 40px; top: 40px; z-index: 5; background-color: #f57c00; }
.later { left: 340px; top: 20px; z-index: 4; background-color: #7b1fa2; }
.context { left: 440px; top: 0; z-index: 1; background-color: #00897b; }
.inside { left: 30px; top: 30px; z-index: 99; background-color: #c2185b; }
.over { left: 500px; top: 50px; z-index: 2; background-color: #455a64; }
</style>

<div class="scene">
  <div class="sq a"></div>
  <div class="sq b"></div>
  <div class="sq c"></div>
  <div class="sq under"></div>
  <div class="block"></div>
  <div class="sq auto">
    <div class="sq escape"></div>
  </div>
  <div class="sq later"></div>
  <div class="sq context">
    <div class="sq inside"></div>
  </div>
  <div class="sq over"></div>
</div>
