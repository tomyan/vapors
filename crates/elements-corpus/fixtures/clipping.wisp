<style>
/* viewport 640x400 */
/* paint: compared with Chromium's pixels in tests/gpu_paint.rs */
/* clipping (F2c S8): overflow hidden and clip cut descendants off at
   the padding box: a wide child, a child's shadow, a clip inside a
   clip, an absolute child placed by the clipping box (cut off) and one
   placed by a box outside it (not), and a bordered clip. Text clipping
   is checked in vapors-text's gpu_text: glyph edges differ from
   Chromium's by more than these rules allow. */
body { margin: 0; padding: 16px; background-color: #e9ecef; display: flex; flex-wrap: wrap; gap: 24px; }
.frame { width: 100px; height: 80px; background-color: #ffffff; overflow: hidden; }
.cut { overflow: clip; }
.wide { width: 160px; height: 30px; margin-top: 10px; background-color: #1976d2; }
.shadowed { width: 60px; height: 40px; margin: 30px 0 0 60px; background-color: #2e7d32; box-shadow: 8px 8px 12px #171b1f; }
.inner { width: 60px; height: 60px; margin: 10px; background-color: #fbc02d; overflow: hidden; }
.placed { position: relative; }
.abs { position: absolute; left: 70px; top: 50px; width: 60px; height: 60px; background-color: #d32f2f; }
.bordered { border: 6px solid #7b1fa2; }
.outside { position: relative; width: 120px; height: 100px; }
.outside .frame { margin: 0; }
</style>

<div class="frame"><div class="wide"></div></div>
<div class="frame cut"><div class="shadowed"></div></div>
<div class="frame"><div class="inner"><div class="wide"></div></div></div>
<div class="frame placed"><div class="abs"></div></div>
<div class="outside"><div class="frame"><div class="abs"></div></div></div>
<div class="frame bordered"><div class="wide"></div></div>
