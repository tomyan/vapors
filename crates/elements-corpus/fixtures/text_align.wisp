<style>
/* viewport 640x400 */
/* text-align and letter-spacing (F2c S6), with inline elements inside
   aligned and spaced lines. */
/* tolerance 0.1 */
/* As in inline_text.wisp: Chromium sums advances in 1/64 px, Parley in
   floats. */
body { font-size: 16px; }
.column { width: 320px; }
.center { text-align: center; }
.right { text-align: right; }
.justify { text-align: justify; }
.wide { letter-spacing: 2px; }
.tight { letter-spacing: -0.5px; }
</style>

<div class="column">
  <p class="center">A centred line with <b>bold words</b> in it.</p>
  <p class="right">Aligned to the <span>right edge</span>, over two lines because it is long.</p>
  <p class="justify">Justified text stretches every line but the last to the full width of the column, <b>spaces and all</b>.</p>
  <p class="wide">Letters <span class="tight">set tighter</span> inside wider ones.</p>
</div>
