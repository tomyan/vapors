<style>
/* viewport 640x400 */
/* The title attribute in the accessibility tree (docs/tooltips.md), as
   Chromium's has it: the name when nothing else names the node, else
   the description, unless aria-describedby gives one. Widths are set,
   as vapors lays these controls out only at a width of their own;
   buttons are as tall as their content. The look is not compared
   (Q1).
   RELAXED: the button rule sets font-family: inherit, font-size:
   inherit and text-align: left, as names.wisp does and for its reason;
   retire them with names.wisp's. */
body { margin: 0; font-size: 14px; }
svg { display: block; }
.ctl { height: 20px; }
.row { display: flex; align-items: flex-start; height: 40px; }
button, .ctl { box-sizing: border-box; width: 80px; margin: 0; padding: 0; border: 1px solid #555; font-family: inherit; font-size: inherit; text-align: left; }
</style>

<div class="row"><button id="open" title="Open (o)">Open</button><button id="undo" title="Undo (Ctrl+Z)"></button><button id="redo" aria-label="Redo" title="Redo (Ctrl+Shift+Z)">R</button><button id="blank" title="">Blank</button><button id="spaced" title="  Spaced
   out  "></button><button id="by" aria-labelledby="lbl" title="By title">B</button></div><div class="row"><span id="lbl">Labelled</span><span id="hint">Hint text</span><button id="hinted" aria-describedby="hint" title="Not shown">Hinted</button><button id="nohint" aria-describedby="nowhere" title="Falls back">Nohint</button><div id="level" class="ctl" role="slider" aria-valuenow="5" title="Level" tabindex="0"></div><button id="icon" title="Export (e)"><svg role="img" aria-label="Export" width="12" height="12" viewBox="0 0 12 12"><path d="M2 2H10V10H2Z" fill="none" stroke="currentColor" stroke-width="1.5"/></svg></button></div><div class="row"><button id="ws" title="   ">WS</button><button id="wsempty" title="   "></button><button id="descspaced" title=" a
  b ">Desc</button></div>
