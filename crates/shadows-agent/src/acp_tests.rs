//! ACP metadata and session-cost boundary tests.
use super::*;
use crate::events::HarnessEvent;

#[test]
fn absent_and_empty_tool_policies_are_distinct() {
    assert!(SessionSetup::default().meta().is_none());
    let setup = SessionSetup {
        tools: Some(vec![]),
        ..Default::default()
    };
    let meta = serde_json::to_value(setup.meta().unwrap()).unwrap();
    assert_eq!(
        meta["claudeCode"]["options"]["tools"],
        serde_json::json!([])
    );
    assert_eq!(
        meta["claudeCode"]["options"]["settingSources"],
        serde_json::json!([])
    );
    assert_eq!(meta["claudeCode"]["options"]["strictMcpConfig"], true);
    let defaults = SessionSetup {
        allowed_tools: vec!["mcp__shadows__*".into()],
        ..Default::default()
    };
    let meta = serde_json::to_value(defaults.meta().unwrap()).unwrap();
    assert!(meta["claudeCode"]["options"].get("tools").is_none());
}

#[test]
fn aylul_meta_loads_only_user_settings() {
    let setup = SessionSetup {
        setting_sources: Some(vec!["user".into()]),
        ..Default::default()
    };
    let meta = serde_json::to_value(setup.meta().unwrap()).unwrap();
    assert_eq!(
        meta["claudeCode"]["options"]["settingSources"],
        serde_json::json!(["user"])
    );
    assert!(meta.get("systemPrompt").is_none());
}

#[test]
fn a_server_error_mentioning_connection_closed_is_still_an_rpc_error() {
    let error = agent_client_protocol::Error::new(-32603, "connection closed by policy");
    assert!(matches!(
        AcpError::from_rpc(error),
        AcpError::Rpc(message) if message == "connection closed by policy"
    ));
}

fn mapped(update: serde_json::Value) -> HarnessEvent {
    use std::sync::Mutex;
    let event = Mutex::new(None);
    let update = serde_json::from_value(update).unwrap();
    mapping::Mapping::forward(&|item| *event.lock().unwrap() = Some(item), update);
    event.into_inner().unwrap().expect("mapped event")
}

#[test]
fn a_tool_call_carries_its_kind_locations_input_and_diffs() {
    use crate::events::{CallKind, ToolDiff, ToolLocation};
    let event = mapped(serde_json::json!({
        "sessionUpdate": "tool_call", "toolCallId": "call-1", "title": "Edit",
        "kind": "edit", "status": "in_progress",
        "locations": [{"path": "/work/a.rs", "line": 4}],
        "rawInput": {"file_path": "/work/a.rs"},
        "content": [{"type": "diff", "path": "/work/a.rs", "oldText": "a", "newText": "b"}]
    }));
    let HarnessEvent::ToolCall { facts, status, .. } = event else {
        panic!("expected tool call")
    };
    assert_eq!(status.as_deref(), Some("in_progress"));
    assert_eq!(facts.kind, Some(CallKind::Edit));
    assert_eq!(
        facts.locations,
        Some(vec![ToolLocation {
            path: "/work/a.rs".into(),
            line: Some(4)
        }])
    );
    assert_eq!(
        facts.input,
        Some(serde_json::json!({"file_path": "/work/a.rs"}))
    );
    assert_eq!(
        facts.diffs,
        Some(vec![ToolDiff {
            path: "/work/a.rs".into(),
            old_text: Some("a".into()),
            new_text: "b".into(),
        }])
    );
    assert_eq!(facts.texts, Some(vec![]));
    assert_eq!(facts.output, None);
}

#[test]
fn an_update_carries_only_what_it_names() {
    use crate::events::{ToolFacts, ToolLocation};
    let event = mapped(serde_json::json!({
        "sessionUpdate": "tool_call_update", "toolCallId": "call-1", "title": "changed"
    }));
    let HarnessEvent::ToolCall { facts, .. } = event else {
        panic!("expected tool call")
    };
    assert_eq!(facts, ToolFacts::default());
    let event = mapped(serde_json::json!({
        "sessionUpdate": "tool_call_update", "toolCallId": "call-1",
        "locations": [{"path": "/work/a.rs", "line": 7}],
        "content": [{"type": "diff", "path": "/work/a.rs", "oldText": "a", "newText": "b"}]
    }));
    let HarnessEvent::ToolCall { facts, .. } = event else {
        panic!("expected tool call")
    };
    assert_eq!(
        facts.locations,
        Some(vec![ToolLocation {
            path: "/work/a.rs".into(),
            line: Some(7)
        }])
    );
    assert_eq!(facts.diffs.unwrap().len(), 1);
    assert_eq!(facts.texts, Some(vec![]));
    assert_eq!(facts.kind, None);
    assert_eq!(facts.input, None);
    assert_eq!(facts.output, None);
}

#[test]
fn a_plan_update_becomes_the_task_list() {
    use crate::events::{TodoItem, TodoStatus};
    let event = mapped(serde_json::json!({
        "sessionUpdate": "plan", "entries": [
            {"content": "one", "priority": "high", "status": "pending"},
            {"content": "two", "priority": "medium", "status": "in_progress"},
            {"content": "three", "priority": "low", "status": "completed"}
        ]
    }));
    let HarnessEvent::Todos(items) = event else {
        panic!("expected todos")
    };
    assert_eq!(
        items,
        vec![
            TodoItem {
                content: "one".into(),
                status: TodoStatus::Pending
            },
            TodoItem {
                content: "two".into(),
                status: TodoStatus::InProgress
            },
            TodoItem {
                content: "three".into(),
                status: TodoStatus::Completed
            },
        ]
    );
}

#[test]
fn a_failed_update_carries_its_text_and_raw_output() {
    let event = mapped(serde_json::json!({
        "sessionUpdate": "tool_call_update", "toolCallId": "call-1", "status": "failed",
        "content": [{"type": "content", "content": {"type": "text", "text": "error text"}}],
        "rawOutput": {"exit": 1}
    }));
    let HarnessEvent::ToolCall { facts, status, .. } = event else {
        panic!("expected tool call")
    };
    assert_eq!(status.as_deref(), Some("failed"));
    assert_eq!(facts.texts, Some(vec!["error text".into()]));
    assert_eq!(facts.output, Some(serde_json::json!({"exit": 1})));
}
