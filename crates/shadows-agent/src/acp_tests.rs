//! ACP metadata and session-cost boundary tests.
use super::*;
use crate::events::HarnessEvent;

#[test]
fn async_tasks_are_advertised() {
    let value = serde_json::to_value(connection::capabilities()).unwrap();
    assert_eq!(
        value["_meta"]["jetbrains"]["air"]["capabilities"],
        serde_json::json!(["recommendedValue", "asyncTasks"])
    );
}

#[test]
fn async_task_updates_become_task_events() {
    use std::sync::Mutex;
    let events = Mutex::new(Vec::new());
    let send = |event| events.lock().unwrap().push(event);
    for update in [
        serde_json::json!({"sessionUpdate":"async_task_spawned", "asyncTaskId":"one",
            "name":"job", "taskType":"shell", "description":"run", "canStop":true,
            "showInTranscript":false}),
        serde_json::json!({"sessionUpdate":"async_task_progress", "asyncTaskId":"one"}),
        serde_json::json!({"sessionUpdate":"async_task_state_update", "asyncTaskId":"one",
            "state":"completed", "summary":"done"}),
    ] {
        connection::dispatch_notification(
            &send,
            &Default::default(),
            agent_client_protocol::UntypedMessage::new(
                "session/update",
                serde_json::json!({"sessionId":"s", "update":update}),
            )
            .unwrap(),
        );
    }
    for update in [
        serde_json::json!({"sessionUpdate":"unknown_extension"}),
        serde_json::json!({"sessionUpdate":"agent_message_chunk", "content":{
            "type":"text", "text":"after"
        }}),
    ] {
        connection::dispatch_notification(
            &send,
            &Default::default(),
            agent_client_protocol::UntypedMessage::new(
                "session/update",
                serde_json::json!({"sessionId":"s", "update":update}),
            )
            .unwrap(),
        );
    }
    let events = events.lock().unwrap();
    assert_eq!(events.len(), 4);
    assert!(matches!(&events[3], HarnessEvent::Chunk { text, .. } if text == "after"));
}

#[test]
fn unknown_session_update_probe() {
    let value = serde_json::json!({
        "sessionId": "probe", "update": {"sessionUpdate": "async_task_spawned"}
    });
    let result =
        serde_json::from_value::<agent_client_protocol::v1::schema::SessionNotification>(value);
    assert!(
        result.is_err(),
        "unknown session update must fail typed decoding"
    );
}

#[tokio::test]
async fn stopping_a_task_asks_the_adapter() {
    use agent_client_protocol::schema::ProtocolVersion;
    use agent_client_protocol::v1::schema::{InitializeRequest, InitializeResponse};
    use agent_client_protocol::{Agent, ByteStreams, Client, UntypedMessage, v1::Responder};
    use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

    let (client, server) = tokio::io::duplex(4096);
    let (cr, cw) = tokio::io::split(client);
    let (sr, sw) = tokio::io::split(server);
    tokio::spawn(async move {
        Agent
            .v1()
            .on_receive_request(
                async |_: InitializeRequest, responder: Responder<InitializeResponse>, _| {
                    responder.respond(InitializeResponse::new(ProtocolVersion::V1))
                },
                agent_client_protocol::on_receive_request!(),
            )
            .on_receive_request(
                async |r: UntypedMessage, responder: Responder<serde_json::Value>, _| {
                    assert_eq!(r.method, "_session/async_task/stop");
                    assert_eq!(
                        r.params,
                        serde_json::json!({
                            "sessionId":"session-1", "asyncTaskId":"task-1"
                        })
                    );
                    responder.respond(serde_json::json!({"stopped":true}))
                },
                agent_client_protocol::on_receive_request!(),
            )
            .connect_with(
                ByteStreams::new(sw.compat_write(), sr.compat()),
                |_| async {
                    std::future::pending::<()>().await;
                    #[allow(unreachable_code)]
                    Ok(())
                },
            )
            .await
            .unwrap();
    });
    let (tx, rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        Client
            .v1()
            .connect_with(
                ByteStreams::new(cw.compat_write(), cr.compat()),
                |cx| async {
                    tx.send(cx).ok();
                    std::future::pending::<()>().await;
                    #[allow(unreachable_code)]
                    Ok(())
                },
            )
            .await
            .unwrap();
    });
    let cx = rx.await.unwrap();
    cx.send_request(InitializeRequest::new(ProtocolVersion::V1))
        .block_task()
        .await
        .unwrap();
    let connection = Connection {
        cx,
        capabilities: PromptCapabilities::default(),
        costs: Default::default(),
        permissions: permissions::Permissions::new(|_| {}),
    };
    assert!(connection.stop_task("session-1", "task-1").await.unwrap());
}

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

#[test]
fn mcp_servers_carry_http_headers_and_stdio_env() {
    let setup = SessionSetup {
        mcp: vec![
            McpServerSpec::Http {
                name: "remote".into(),
                url: "https://example.test".into(),
                headers: vec![("Authorization".into(), "secret-header".into())],
            },
            McpServerSpec::Stdio {
                name: "local".into(),
                command: "server".into(),
                args: vec!["--run".into()],
                env: vec![("TOKEN".into(), "secret-env".into())],
            },
        ],
        ..Default::default()
    };
    let actual = serde_json::to_value(setup.mcp_servers()).unwrap();
    assert_eq!(
        actual,
        serde_json::json!([
            {"type":"http", "name":"remote", "url":"https://example.test",
             "headers":[{"name":"Authorization", "value":"secret-header"}]},
            {"name":"local", "command":"server", "args":["--run"],
             "env":[{"name":"TOKEN", "value":"secret-env"}]}
        ])
    );
}

#[test]
fn mcp_spec_debug_never_prints_values() {
    let specs = vec![
        McpServerSpec::Http {
            name: "remote".into(),
            url: "url".into(),
            headers: vec![("auth".into(), "secret-header".into())],
        },
        McpServerSpec::Stdio {
            name: "local".into(),
            command: "server".into(),
            args: vec!["secret-arg".into()],
            env: vec![("key".into(), "secret-env".into())],
        },
    ];
    let debug = format!("{specs:?}");
    assert!(debug.contains("remote") && debug.contains("local"));
    assert!(!debug.contains("secret-header") && !debug.contains("secret-env"));
    assert!(!debug.contains("secret-arg"));
}

fn frames(updates: Vec<serde_json::Value>) -> Vec<HarnessEvent> {
    use agent_client_protocol::v1::schema::SessionUpdate;
    let events = std::sync::Mutex::new(Vec::new());
    let send = |event| events.lock().unwrap().push(event);
    for update in updates {
        let update: SessionUpdate = serde_json::from_value(update).unwrap();
        crate::acp::mapping::Mapping::forward(&send, update);
    }
    events.into_inner().unwrap()
}

fn compacted(events: &[HarnessEvent]) -> Vec<Option<String>> {
    events
        .iter()
        .filter_map(|e| match e {
            HarnessEvent::Compacted { trigger } => Some(trigger.clone()),
            _ => None,
        })
        .collect()
}

fn compact_meta(trigger: &str) -> serde_json::Value {
    serde_json::json!({"contextCompaction": {"version": 1, "trigger": trigger},
        "claudeCode": {"toolName": "compact"}})
}

#[test]
fn a_completed_compaction_is_reported_with_its_trigger() {
    let events = frames(vec![
        serde_json::json!({"sessionUpdate":"tool_call", "toolCallId":"c", "title":"Compact",
            "kind":"think", "status":"in_progress",
            "_meta":{"contextCompaction":{"version":1}}}),
        serde_json::json!({"sessionUpdate":"tool_call_update", "toolCallId":"c",
            "status":"completed", "rawOutput":{"trigger":"manual"},
            "_meta": compact_meta("manual")}),
    ]);
    assert!(matches!(&events[1], HarnessEvent::ToolCall { .. }));
    assert!(matches!(&events[2], HarnessEvent::Compacted { trigger }
        if trigger.as_deref() == Some("manual")));
    assert_eq!(events.len(), 3);
}

#[test]
fn a_compaction_opened_already_terminal_is_reported() {
    let events = frames(vec![
        serde_json::json!({"sessionUpdate":"tool_call", "toolCallId":"c", "title":"Compact",
            "status":"completed", "rawOutput":{"trigger":"automatic"},
            "_meta":{"contextCompaction":{"version":1}}}),
    ]);
    assert_eq!(compacted(&events), vec![Some("automatic".to_owned())]);
}

#[test]
fn a_failed_compaction_is_not_reported() {
    let events = frames(vec![
        serde_json::json!({"sessionUpdate":"tool_call_update", "toolCallId":"c",
            "status":"failed", "_meta": compact_meta("manual")}),
    ]);
    assert!(compacted(&events).is_empty());
}

#[test]
fn a_repeated_metadata_frame_is_not_reported_again() {
    let events = frames(vec![
        serde_json::json!({"sessionUpdate":"tool_call_update", "toolCallId":"c",
            "status":"completed", "_meta": compact_meta("manual")}),
        serde_json::json!({"sessionUpdate":"tool_call_update", "toolCallId":"c",
            "_meta": compact_meta("manual")}),
    ]);
    assert_eq!(compacted(&events).len(), 1);
}

#[test]
fn an_ordinary_completed_tool_is_not_a_compaction() {
    let events = frames(vec![
        serde_json::json!({"sessionUpdate":"tool_call_update", "toolCallId":"t",
            "status":"completed", "_meta":{"claudeCode":{"toolName":"Read"}}}),
    ]);
    assert!(compacted(&events).is_empty());
}
