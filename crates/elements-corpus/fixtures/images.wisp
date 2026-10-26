<style>
/* viewport 640x400 390x800 */
/* img (F2c S10): block images at their natural size, sized by one
   length through the aspect ratio, by both, by a percentage, capped by
   max-width; images without src, bare and sized; images in a flex
   row, where they are blockified; border-box sizing with padding; an
   author's aspect-ratio; min-height and max-height through the ratio;
   images stretched by align-items in a flex row; grid items; and a
   percentage width in a shrink-to-fit parent. */
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
.boxed { box-sizing: border-box; padding: 5px; border: 2px solid #333; }
.square { aspect-ratio: 1; width: 40px; }
.boxed-width { box-sizing: border-box; width: 60px; padding: 5px; }
.boxed-max { box-sizing: border-box; max-height: 20px; padding: 4px; }
.short { min-height: 50px; }
.narrow { max-height: 10px; }
.stretch { display: flex; height: 60px; }
.grid { display: grid; grid-template-columns: 100px 1fr; }
.fit { position: absolute; left: 300px; top: 8px; }
.fit img { width: 50%; }
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
<img class="boxed" src="wide.png">
<img class="square" src="wide.png">
<img class="boxed-width" src="wide.png">
<img class="boxed-max" src="wide.png">
<img class="short" src="wide.png">
<img class="narrow" src="wide.png">
<div class="stretch"><img src="wide.png"><img src="tall.png"></div>
<div class="grid"><img src="wide.png"><img src="tall.png"></div>
<div class="fit"><img src="wide.png"></div>
