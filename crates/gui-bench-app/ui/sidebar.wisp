<script>
use crate::nav_item::{NavItem, NavItemProps};
use crate::dashboard::{APP_NAME, BADGE_COUNT, BADGE_ITEM, NAV_ITEMS, USER_INITIALS, USER_NAME, USER_ROLE};
let (app_name, initials, user_name, user_role) = (APP_NAME, USER_INITIALS, USER_NAME, USER_ROLE);
#[prop] let active: Signal<usize> = Signal::new(0);
let items: Vec<(usize, String, u32)> = NAV_ITEMS
    .iter()
    .enumerate()
    .map(|(i, label)| (i, (*label).to_string(), if i == BADGE_ITEM { BADGE_COUNT } else { 0 }))
    .collect();
</script>

<style>
.sidebar { position: relative; width: 240px; height: 100%; flex: none; background-color: #111827; }
.brand { position: absolute; left: 20px; top: 18px; height: 28px; display: flex; align-items: center; gap: 12px; color: #ffffff; font-size: 16px; font-weight: 600; }
.logo { width: 28px; height: 28px; border-radius: 8px; background-color: #6366f1; }
.nav { position: absolute; left: 12px; top: 80px; width: 216px; }
.user { position: absolute; left: 0; top: 720px; width: 240px; height: 80px; box-sizing: border-box; padding: 19px 20px 0; border-top: 1px solid #1f2937; display: flex; gap: 12px; }
.user-avatar { width: 32px; height: 32px; flex: none; border-radius: 16px; background-color: #f59e0b; color: #ffffff; font-size: 13px; font-weight: 600; display: flex; align-items: center; justify-content: center; }
.who { height: 32px; display: flex; flex-direction: column; justify-content: center; }
.user-name { color: #ffffff; font-size: 14px; line-height: 18px; }
.user-role { color: #9ca3af; font-size: 12px; line-height: 14px; }
</style>

<div class="sidebar">
  <div class="brand"><div class="logo"></div>{app_name}</div>
  <nav class="nav">
    {for item in items.clone() key=item.0}
      <NavItem index={item.get().0} label={item.get().1} badge={item.get().2} active={active} />
    {/for}
  </nav>
  <div class="user">
    <div class="user-avatar">{initials}</div>
    <div class="who"><div class="user-name">{user_name}</div><div class="user-role">{user_role}</div></div>
  </div>
</div>
