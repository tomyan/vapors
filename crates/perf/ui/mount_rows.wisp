<script>
use crate::mount_row::{MountRow, MountRowProps};

#[prop]
let rows: Signal<Vec<(u32, String, String)>> = Signal::new(Vec::new());
</script>

{for row in rows.get() key=row.0}<MountRow label={row.get().1} value={row.get().2} marked={false} />{/for}
