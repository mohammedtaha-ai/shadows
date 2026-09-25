//! ACP request debug output can contain a live MCP bearer. The daemon's
//! tracing policy must cap that crate even when RUST_LOG asks for trace.

#[test]
fn acp_request_debug_is_excluded_from_console_and_debug_file() {
    // This test binary has one test and sets the process environment before
    // installing its one global subscriber.
    unsafe { std::env::set_var("RUST_LOG", "trace") };
    let tmp = tempfile::tempdir().unwrap();
    let log = shadows::tracing::init(false, Some(tmp.path()))
        .unwrap()
        .expect("debug file");
    let path = log.path().to_path_buf();

    assert!(!tracing::enabled!(
        target: "agent_client_protocol::jsonrpc::outgoing_actor",
        tracing::Level::DEBUG
    ));
    assert!(tracing::enabled!(
        target: "agent_client_protocol::jsonrpc::outgoing_actor",
        tracing::Level::INFO
    ));
    assert!(tracing::enabled!(
        target: "shadows::acp_log_safety",
        tracing::Level::DEBUG
    ));

    tracing::debug!(
        target: "agent_client_protocol::jsonrpc::outgoing_actor",
        request = "Bearer synthetic-secret",
        "outgoing_protocol_actor"
    );
    tracing::debug!(target: "shadows::acp_log_safety", "shadows-debug-visible");
    drop(log);
    let text = std::fs::read_to_string(path).unwrap();
    assert!(text.contains("shadows-debug-visible"));
    assert!(!text.contains("synthetic-secret"));
}
