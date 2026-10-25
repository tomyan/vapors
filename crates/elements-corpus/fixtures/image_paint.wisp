<style>
/* viewport 640x400 */
/* paint: compared with Chromium's pixels in tests/gpu_paint.rs */
/* img in paint (F2c S10): images of four coloured quadrants at their
   natural size, stretched by a width, inside padding and a border, and
   cut off by an overflow: hidden box; and 1 px stripes shrunk about
   3x and 9x, which alias without a filter over the source. */
body { margin: 0; padding: 16px; background-color: #e9ecef; display: flex; flex-wrap: wrap; gap: 20px; align-items: flex-start; }
img { display: block; }
.stretched { width: 120px; }
.framed { padding: 6px; border: 4px solid #171b1f; background-color: #ffffff; }
.cut { width: 30px; height: 15px; overflow: hidden; }
.cut img { width: 80px; }
.small { width: 19px; }
.smaller { width: 7px; }
</style>

<img src="wide.png">
<img src="tall.png">
<img class="stretched" src="wide.png">
<img class="framed" src="wide.png">
<div class="cut"><img src="wide.png"></div>
<img class="small" src="stripes.png">
<img class="smaller" src="stripes.png">
