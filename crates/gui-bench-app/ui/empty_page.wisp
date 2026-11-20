<script>
use crate::dashboard::EMPTY_LABEL;
let label = EMPTY_LABEL.to_string();
</script>

<style>
/* Its top is the content padding plus 48 below the top bar (EMPTY_TOP). */
.empty { flex: none; margin-top: 48px; text-align: center; font-size: 14px; line-height: 20px; color: #6b7280; }
</style>

<div class="empty">{label}</div>
