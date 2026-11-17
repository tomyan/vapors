use super::scope;

#[test]
fn the_scope_carries_the_memory_and_task_limits_without_swap() {
    let args: Vec<String> = scope()
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    for property in ["MemoryMax=32G", "MemorySwapMax=0", "TasksMax=4096"] {
        assert!(
            args.windows(2).any(|w| w == ["-p", property]),
            "{property} missing: {args:?}"
        );
    }
    assert_eq!(args.last().map(String::as_str), Some("--"));
}
