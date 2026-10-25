// The .events player's page side (events_play.rs). __TYPES__ is the
// script's `listen` list. Each event of those types is kept when it
// reaches the window in the capture phase, so every one is seen once,
// whatever stops it later; its fields are read when the log is drained,
// after it has run, so defaultPrevented is final.
(() => {
  const types = __TYPES__;
  const log = [];
  // tag#id, else the full child-index path from body (impl-f2c, D3).
  const name = (el) => {
    if (!el || el === document.body || el === document.documentElement) return el ? el.localName : null;
    if (el.id) return el.localName + "#" + el.id;
    const index = Array.prototype.indexOf.call(el.parentElement.children, el) + 1;
    return name(el.parentElement) + ">" + el.localName + ":nth-child(" + index + ")";
  };
  for (const type of types) window.addEventListener(type, (event) => log.push(event), true);
  window.__eventsDrain = () => log.splice(0).map((e) => {
    const entry = { type: e.type, target: name(e.target), bubbles: e.bubbles, defaultPrevented: e.defaultPrevented };
    if ("pointerType" in e && e.pointerType) entry.pointerType = e.pointerType;
    if ("key" in e && e.key) entry.key = e.key;
    return entry;
  });
  window.__eventsFocus = () => {
    const el = document.activeElement;
    return { focus: name(el), focusVisible: !!el && el !== document.body && el.matches(":focus-visible") };
  };
  window.__eventsCount = (target) => document.querySelectorAll(target).length;
  return true;
})()
