<style>
/* viewport 640x400 */
/* Icons in the accessibility tree (docs/icons.md), as Chromium's has
   them: an aria-hidden svg leaves a named button its own name and is
   not in the tree; a role="img" svg is an image named by its
   aria-label, alone and inside a button with text, whose name from
   content takes the label; a named icon that is visibility: hidden is
   not in the tree and adds nothing to its button's name (Chromium 155,
   2026-11-10, first recorded by the hidden-icon-name probe this
   fixture replaced).
   RELAXED: the button rule sets font-family: inherit, font-size:
   inherit and text-align: left, as names.wisp does and for its reason:
   vapors' ua.css lacks Chromium's button font and centred text. Retire
   them with names.wisp's. */
body { margin: 0; font-size: 14px; }
.row { display: flex; align-items: flex-start; gap: 8px; }
button { display: flex; box-sizing: border-box; width: 80px; margin: 0; padding: 0; border: 1px solid #555; font-family: inherit; font-size: inherit; text-align: left; }
svg { display: block; }
.unseen { visibility: hidden; }
</style>

<div class="row"><button id="undo" aria-label="Undo"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M8 10V2M5 5l3-3 3 3M3 9v5h10V9" fill="none" stroke="currentColor" stroke-width="1.5"/></svg></button><button id="open"><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M8 10V2M5 5l3-3 3 3M3 9v5h10V9" fill="none" stroke="currentColor" stroke-width="1.5"/></svg>Open</button><svg id="alone" viewBox="0 0 16 16" width="16" height="16" role="img" aria-label="Export"><path d="M8 10V2M5 5l3-3 3 3M3 9v5h10V9" fill="none" stroke="currentColor" stroke-width="1.5"/></svg><button id="shown"><svg viewBox="0 0 16 16" width="16" height="16" role="img" aria-label="Export"><path d="M8 10V2M5 5l3-3 3 3M3 9v5h10V9" fill="none" stroke="currentColor" stroke-width="1.5"/></svg>Go</button><button id="hidden"><svg class="unseen" viewBox="0 0 16 16" width="16" height="16" role="img" aria-label="Export"><path d="M8 10V2M5 5l3-3 3 3M3 9v5h10V9" fill="none" stroke="currentColor" stroke-width="1.5"/></svg>Go</button></div>
