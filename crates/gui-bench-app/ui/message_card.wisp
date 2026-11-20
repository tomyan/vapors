<script>
use crate::dashboard::{message, AVATAR_COLORS};
#[prop] let index: usize = 0;
let found = message(index);
let (author, initials, time, body) = (found.author, found.initials, found.time, found.body);
let colour = AVATAR_COLORS.iter().position(|c| *c == found.avatar).expect("avatar colour not in AVATAR_COLORS");
</script>

<style>
.message { position: relative; flex: none; display: flex; gap: 12px; box-sizing: border-box; padding: 16px 20px; border-radius: 12px; background-color: #ffffff; box-shadow: 0 4px 12px #1118271a; }
.message-avatar { flex: none; width: 36px; height: 36px; border-radius: 18px; color: #ffffff; font-size: 13px; font-weight: 600; display: flex; align-items: center; justify-content: center; }
/* One class per entry of AVATAR_COLORS (a test holds each to the constant). */
.c0 { background-color: #f87171; } .c1 { background-color: #fb923c; } .c2 { background-color: #fbbf24; } .c3 { background-color: #34d399; }
.c4 { background-color: #22d3ee; } .c5 { background-color: #60a5fa; } .c6 { background-color: #a78bfa; } .c7 { background-color: #f472b6; }
.message-text { flex: 1; min-width: 0; }
.message-head { display: flex; align-items: baseline; gap: 8px; height: 20px; line-height: 20px; }
.author { font-size: 14px; font-weight: 600; color: #111827; }
.time { font-size: 13px; color: #6b7280; }
.body { font-size: 14px; line-height: 20px; color: #111827; }
</style>

<div class="message">
  <div class="message-avatar" class:c0={colour == 0} class:c1={colour == 1} class:c2={colour == 2} class:c3={colour == 3} class:c4={colour == 4} class:c5={colour == 5} class:c6={colour == 6} class:c7={colour == 7}>{initials}</div>
  <div class="message-text">
    <div class="message-head"><span class="author">{author}</span><span class="time">{time}</span></div>
    <div class="body">{body}</div>
  </div>
</div>
