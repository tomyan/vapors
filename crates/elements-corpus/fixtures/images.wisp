<style>
/* viewport 640x400 390x800 */
/* img (F2c S10): block images at their natural size, sized by one
   length through the aspect ratio, by both, by a percentage, capped by
   max-width; images without src, bare and sized; and images in a flex
   row, where they are blockified. */
body { margin: 8px; }
img { display: block; margin-bottom: 4px; }
.wide-by-width { width: 80px; }
.tall-by-height { height: 60px; }
.both { width: 50px; height: 50px; }
.half { width: 50%; }
.capped { width: 300px; max-width: 120px; }
.sized { width: 30px; height: 10px; }
.row { display: flex; gap: 6px; align-items: flex-start; }
.row .thumb { height: 24px; }
</style>

<img src="wide.png">
<img src="tall.png">
<img class="wide-by-width" src="wide.png">
<img class="tall-by-height" src="tall.png">
<img class="both" src="wide.png">
<img class="half" src="wide.png">
<img class="capped" src="wide.png">
<img>
<img class="sized">
<div class="row"><img src="wide.png"><img class="thumb" src="tall.png"><img src="tall.png"></div>
