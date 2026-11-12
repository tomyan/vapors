<script>
use super::dynamic_children_button::{DynamicChildrenButton, DynamicChildrenButtonProps};

let show: Signal<bool> = Signal::new(false);
</script>

<DynamicChildrenButton show={show}>{if show.get()}<span>showing</span>{else}<span>hidden</span>{/if}</DynamicChildrenButton>
