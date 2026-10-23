<style>
/* viewport 640x400 */
/* paint: compared with Chromium's pixels in tests/gpu_paint.rs */
body { margin: 0; padding: 16px; background-color: #e9ecef; display: flex; flex-wrap: wrap; gap: 20px; }
.plain { width: 80px; height: 60px; background-color: #171b1f; }
.sides { width: 80px; height: 60px; background-color: #ffffff; border-top: 2px solid #d32f2f; border-right: 4px solid #388e3c; border-bottom: 6px solid #1976d2; border-left: 8px solid #fbc02d; }
.round { width: 80px; height: 60px; background-color: #3a7bd5; border-radius: 12px; }
.ring { width: 60px; height: 60px; border: 3px solid #171b1f; border-radius: 9999px; }
.pill { width: 120px; height: 32px; background-color: #2e7d32; border-radius: 9999px; }
.card { width: 120px; height: 70px; background-color: #ffffff; border-radius: 8px; box-shadow: 0 4px 12px #00000059; }
.hard { width: 80px; height: 60px; background-color: #ffffff; box-shadow: 6px 6px 0 #171b1f; }
.spread { width: 80px; height: 60px; background-color: #ffffff; border-radius: 4px; box-shadow: 0 0 0 4px #3a7bd5; }
.soft { width: 80px; height: 60px; background-color: #ffffff; border-radius: 8px; }
.corner { width: 60px; height: 60px; background-color: #3a7bd5; border-top-left-radius: 40px; }
.mixed { width: 100px; height: 40px; background-color: #2e7d32; border-radius: 20px 9999px 9999px 20px; }
</style>

<div class="plain"></div>
<div class="sides"></div>
<div class="round"></div>
<div class="ring"></div>
<div class="pill"></div>
<div class="card"></div>
<div class="hard"></div>
<div class="spread"></div>
<div class="soft"></div>
<div class="corner"></div>
<div class="mixed"></div>
