pub fn is_debug_mode() -> bool {
    std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "".to_string())
        .contains("debug")
}
