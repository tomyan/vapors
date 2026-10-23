<style>
/* viewport 640x400 390x800 */
/* Inline formatting contexts (F2c S6): spans, bold and monospace runs
   in one paragraph, whitespace collapsed across element boundaries, an
   anonymous block beside a block, and inline boxes that wrap.
   code names Geist Mono: Chromium's generic monospace is the system's
   (DejaVu Sans Mono here), where vapors' is the bundled Geist Mono.
 */
/* tolerance 0.1 */
/* Inline boxes' edges are sums of glyph advances: Chromium sums them in
   1/64 px layout units, Parley in floats, so they differ by up to 0.06
   px over a run here (2026-10-23, Chromium 155). */
body { font-size: 16px; }
.column { width: 300px; }
.note { font-size: 14px; line-height: 20px; }
code { font-family: "Geist Mono", monospace; background-color: #eceff1; }
</style>

<div class="column">
  <p>Curves set from a <strong>neutral sample</strong>, applied as <span class="layer">layers you can <em>turn off</em></span>.</p>
  <p class="note">A white point of <code>R 238 G 231 B 222</code> at <b>x 1024 y 768</b>,   with   spaces    collapsed.</p>
  <div>Loose text beside a block<div>The block</div>and after it.</div>
  <p><span>Nested <b>bold <code>mono</code> text</b> in a span</span> that wraps over the end of the line here.</p>
</div>
