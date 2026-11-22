<script>
use crate::dashboard::{message, AVATAR_COLORS};
use crate::data_colours::avatar;
#[prop] let index: usize = 0;
let found = message(index);
let (author, initials, time, body) = (found.author, found.initials, found.time, found.body);
let colour = AVATAR_COLORS.iter().position(|c| *c == found.avatar).expect("avatar colour not in AVATAR_COLORS");
let avatar_back = avatar(colour);
</script>

<style>
.message { position: relative; flex: none; display: flex; gap: 12px; box-sizing: border-box; padding: 16px 20px; border-radius: 12px; background-color: #ffffff; box-shadow: 0 4px 12px #1118271a; }
.message-avatar { flex: none; width: 36px; height: 36px; border-radius: 18px; color: #ffffff; font-size: 13px; font-weight: 600; display: flex; align-items: center; justify-content: center; }
/* The avatar background is bound: style:background-color of the palette entry (data_colours.rs). */
.message-text { flex: 1; min-width: 0; }
.message-head { display: flex; align-items: baseline; gap: 8px; height: 20px; line-height: 20px; }
.author { font-size: 14px; font-weight: 600; color: #111827; }
.time { font-size: 13px; color: #6b7280; }
.body { font-size: 14px; line-height: 20px; color: #111827; }
</style>

<div class="message">
  <div class="message-avatar" style:background-color={avatar_back}>{initials}</div>
  <div class="message-text">
    <div class="message-head"><span class="author">{author}</span><span class="time">{time}</span></div>
    <div class="body">{body}</div>
  </div>
</div>
