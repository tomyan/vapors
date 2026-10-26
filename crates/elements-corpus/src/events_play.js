// The .events player's page side (events_play.rs). __TYPES__ is the
// script's `listen` list. Each event of those types is kept when it
// reaches the window in the capture phase, so every one is seen once,
// whatever stops it later; its fields are read when the log is drained,
// after it has run, so defaultPrevented is final.
(() => {
  const types = __TYPES__;
  const log = [];
  // tag#id, else the full child-index path from body (impl-f2c, D3);
  // the window, the document and an element no longer in it by name.
  const name = (t) => {
    if (t === window) return "window";
    if (t === document) return "document";
    if (!(t instanceof Element)) return "(" + (t && t.nodeName || String(t)) + ")";
    if (t === document.documentElement || t === document.body) return t.localName;
    if (t.id) return t.localName + "#" + t.id;
    if (!t.parentElement) return "(removed " + t.localName + ")";
    const index = Array.prototype.indexOf.call(t.parentElement.children, t) + 1;
    return name(t.parentElement) + ">" + t.localName + ":nth-child(" + index + ")";
  };
  for (const type of types) window.addEventListener(type, (event) => log.push(event), true);
  // Set by the player's settle once the last step's log is taken
  // (events_play.rs, SETTLE); false at each injection, so a page that
  // waits for it acts after the last step of this play only.
  window.__eventsSettling = false;
  window.__eventsDrain = () => log.splice(0).map((e) => {
    const entry = { type: e.type, target: name(e.target), bubbles: e.bubbles, defaultPrevented: e.defaultPrevented };
    if ("pointerType" in e && e.pointerType) entry.pointerType = e.pointerType;
    if ("key" in e && e.key) entry.key = e.key;
    return entry;
  });
  window.__eventsFocus = () => {
    const el = document.activeElement;
    return { focus: el ? name(el) : null, focusVisible: !!el && el !== document.body && el.matches(":focus-visible") };
  };
  window.__eventsCount = (target) => target === "window" || target === "document" ? 1 : document.querySelectorAll(target).length;
  return true;
})()
