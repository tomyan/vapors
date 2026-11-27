<style>
/* viewport 640x400 */
/* paint: compared with Chromium's pixels in tests/gpu_paint.rs */
/* rounded clips (flat-scene F3.2b): overflow hidden with a border-radius
   cuts descendants off at the padding box with its radii (the radius
   less the border): a wide child across the top corners, the same in a
   bordered box, a child's shadow across a corner, a clipping box clear
   of the corners (square), and a line of text clear of them. A click
   inside the bordered box's border curve but outside its padding curve
   misses the child (rounded_clip.events). */
body { margin: 0; padding: 16px; background-color: #e9ecef; display: flex; flex-wrap: wrap; gap: 24px; font-size: 14px; }
.card { width: 100px; height: 80px; background-color: #ffffff; overflow: hidden; border-radius: 16px; }
.wide { width: 160px; height: 30px; background-color: #1976d2; }
.bordered { border: 4px solid #7b1fa2; border-radius: 20px; }
.shadowed { width: 60px; height: 40px; margin: 30px 0 0 50px; background-color: #2e7d32; box-shadow: 8px 8px 12px #171b1f; }
.inner { height: 30px; margin-top: 20px; background-color: #fbc02d; overflow: hidden; }
.line { margin: 30px 0 0 20px; color: #212529; }
</style>

<div class="card"><div class="wide"></div></div>
<div class="card bordered" id="bordered"><div class="wide" id="wide"></div></div>
<div class="card"><div class="shadowed"></div></div>
<div class="card"><div class="inner"><div class="wide"></div></div></div>
<div class="card"><div class="line">clipped</div></div>
