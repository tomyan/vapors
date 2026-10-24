<style>
/* viewport 640x400 390x800 */
/* position (F2c S8): relative offsets, absolute boxes placed by their
   insets against the nearest positioned ancestor (in px, %, and both
   sides at once), one left at its static position, and fixed boxes
   placed against the viewport. */
body { margin: 8px; font-size: 16px; }
.frame { position: relative; width: 300px; height: 160px; padding: 10px; background-color: #eef; }
.nudged { position: relative; top: 6px; left: -4px; height: 20px; background-color: #cdf; }
.corner { position: absolute; top: 4px; right: 8px; width: 40px; height: 30px; background-color: #fcc; }
.stretched { position: absolute; left: 10%; right: 10%; bottom: 12px; height: 18px; background-color: #cfc; }
.static-spot { position: absolute; width: 50px; height: 10px; background-color: #ffc; }
.after { height: 24px; background-color: #ddd; }
.toast { position: fixed; right: 16px; bottom: 16px; width: 180px; height: 48px; background-color: #333; }
.pinned { position: fixed; top: 0; left: 0; width: 20px; height: 20px; background-color: #f80; }
</style>

<div class="frame">
  <div class="nudged"></div>
  <div class="corner"></div>
  <div class="stretched"></div>
  <div class="static-spot"></div>
  <div class="after"></div>
</div>
<div class="after"></div>
<div class="toast"></div>
<div class="pinned"></div>
