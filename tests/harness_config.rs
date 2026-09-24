//! Spec §1.4's one rule, and the block shapes the fixture could not carry.
//!
//! Separate from `tests/harness_stream.rs`, whose job is the classifier against
//! a real captured turn. These two tests are about inputs a real turn on this
//! machine did not produce: a misconfigured harness path, and durable lines
//! carrying tool blocks. The shapes come from the measured examples in
//! `docs/evidence/harness/SERVE_STREAM_SPIKE.md`, not from invention.

use shadows::agent::{AgentHarness, StreamItem, claude::ClaudeHarness};
use shadows::config::{ConfigError, adapter_version, harness_path};

/// Spec §1.4: the harness is resolved from explicit configuration, never from
/// `PATH`. `claude` is what a PATH lookup looks like when it is spelled as a
/// path, and it was the CLI's default until this test existed.
///
/// What it costs if this regresses: an Operation records the harness path and
/// version it ran. If the path means "whatever PATH resolved at spawn time",
/// that record says nothing, and an auto-update can change the stream contract
/// underneath a running install with no signal.
#[test]
fn a_bare_program_name_is_refused_as_a_harness_path() {
    for bare in ["claude", "claude.exe", "./claude", "bin/claude"] {
        let err = harness_path(std::path::Path::new(bare))
            .expect_err("a non-absolute harness path must be refused");
        assert_eq!(err, ConfigError::HarnessNotAbsolute(bare.to_string()));
    }

    let dir = tempfile::tempdir().unwrap();
    let absolute = dir.path().join("claude");
    std::fs::write(&absolute, "").unwrap();
    assert_eq!(harness_path(&absolute).unwrap(), absolute);
    let missing = dir.path().join("missing-claude");
    assert_eq!(
        harness_path(&missing).unwrap_err(),
        ConfigError::HarnessNotFound(missing.to_string_lossy().into_owned())
    );
}

#[test]
fn adapter_version_is_read_from_its_package() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("dist")).unwrap();
    std::fs::write(dir.path().join("package.json"), r#"{"version":"0.81.1"}"#).unwrap();
    std::fs::write(dir.path().join("dist/index.js"), "").unwrap();
    assert_eq!(adapter_version(&dir.path().join("dist/index.js")), "0.81.1");
    assert_eq!(adapter_version(&dir.path().join("nope.js")), "unknown");
}

/// The captured fixture is a pure-text turn, so it never exercises the tool and
/// thinking block arms. The evidence report shows both durable tool shapes it
/// measured — an `assistant` line carrying `tool_use`, and a `tool_result`
/// arriving as role `user` — so they are tested from those shapes.
///
/// Task 9 runs a real Planner turn, which invokes tools. A swap between these
/// arms would mislabel every tool call in the durable history while the whole of
/// `tests/harness_stream.rs` stayed green.
#[test]
fn durable_tool_blocks_are_each_named_distinctly() {
    let h = ClaudeHarness::new("claude".into(), "2.1.278".into());

    let tool_use = r#"{"type":"assistant","uuid":"u-1","message":{"role":"assistant",
        "content":[{"type":"tool_use","id":"toolu_1","name":"Read","input":{}}]}}"#;
    match h.classify(tool_use) {
        StreamItem::Entry { uuid, role, text } => {
            assert_eq!(uuid, "u-1");
            assert_eq!(role, "assistant");
            assert_eq!(text, "[tool_use: Read]", "the tool's name must survive");
        }
        other => panic!("a tool_use line is durable, got {other:?}"),
    }

    let tool_result = r#"{"type":"user","uuid":"u-2","message":{"role":"user",
        "content":[{"tool_use_id":"toolu_1","type":"tool_result","content":"ok"}]}}"#;
    match h.classify(tool_result) {
        StreamItem::Entry { role, text, .. } => {
            assert_eq!(role, "user", "a tool result arrives as role user");
            assert_eq!(text, "[tool_result]");
        }
        other => panic!("a tool_result line is durable, got {other:?}"),
    }

    let thinking = r#"{"type":"assistant","uuid":"u-3","message":{"role":"assistant",
        "content":[{"type":"thinking","thinking":"..."}]}}"#;
    match h.classify(thinking) {
        StreamItem::Entry { text, .. } => assert_eq!(text, "[thinking]"),
        other => panic!("a thinking line is durable, got {other:?}"),
    }

    // Mixed blocks join in order, so a tool call after text is not swallowed.
    let mixed = r#"{"type":"assistant","uuid":"u-4","message":{"role":"assistant",
        "content":[{"type":"text","text":"before"},
                   {"type":"tool_use","id":"t","name":"Bash","input":{}}]}}"#;
    match h.classify(mixed) {
        StreamItem::Entry { text, .. } => assert_eq!(text, "before\n[tool_use: Bash]"),
        other => panic!("a mixed line is durable, got {other:?}"),
    }
}
