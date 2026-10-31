//! The scripted answers to a prompt.
//!
//! Prompts: ordinary (two chunks), `two-messages` (tool updates),
//! `report` (session state), `/context` (delayed first report), `hang` (cancel),
//! `steerable` (waits for a `_session/steering`, then answers with its text),
//! `ignore-cancel` (never), `exit` (code 3), `ask-permission` (reject),
//! `usage` (two context updates), `refuse` (max_tokens), `title <text>` (names
//! the session `<text>` in a `session_info_update` after answering), and
//! `mcp <tool> <json>` (calls the session's MCP server). `report` also
//! shows what the session opened with, the client capabilities' `_meta` that
//! `initialize` carried, and the prompt's blocks. Models:
//! `fake-large` (efforts, `auto`), `fake-small` (efforts, no `auto`),
//! `fake-tiny` (no effort), `fake-locked` (refused, as an account without
//! credits is). A session starts at `fake-large`, `high`, `auto`: as the real
//! adapter does, at the person's own defaults rather than Shadows'.

use std::time::Duration;

use agent_client_protocol::schema::v1::{
    ContentBlock, ContentChunk, PermissionOption, PermissionOptionKind, RequestPermissionRequest,
    SessionInfoUpdate, SessionNotification, SessionUpdate, StopReason, TextContent, ToolCall,
    ToolCallUpdate, UsageUpdate,
};
use agent_client_protocol::{Client, ConnectionTo};
use serde_json::{Value, json};

use crate::mcp::call_mcp;
use crate::session::Session;

pub(crate) fn update(
    cx: &ConnectionTo<Client>,
    id: &str,
    item: SessionUpdate,
) -> agent_client_protocol::Result<()> {
    cx.send_notification(SessionNotification::new(id.to_owned(), item))
}

fn chunk(
    cx: &ConnectionTo<Client>,
    id: &str,
    mid: &str,
    text: &str,
) -> agent_client_protocol::Result<()> {
    update(
        cx,
        id,
        SessionUpdate::AgentMessageChunk(
            ContentChunk::new(ContentBlock::Text(TextContent::new(text))).message_id(mid),
        ),
    )
}

/// What one prompt is answered with.
pub(crate) struct Turn<'a> {
    pub(crate) id: &'a str,
    pub(crate) prompt: &'a str,
    pub(crate) blocks: &'a [&'a str],
    pub(crate) session: &'a Session,
    pub(crate) first_context: bool,
    pub(crate) resumes: usize,
    pub(crate) client_meta: Value,
}

/// Runs the script for `turn.prompt`; answers the stop reason and, for `title`,
/// an update to send after the response.
pub(crate) async fn answer(
    cx: &ConnectionTo<Client>,
    turn: Turn<'_>,
) -> agent_client_protocol::Result<(StopReason, Option<SessionUpdate>)> {
    let Turn {
        id,
        prompt,
        blocks,
        session: s,
        first_context,
        resumes,
        client_meta,
    } = turn;
    match prompt {
        "todo-list" => {
            for status in ["pending", "completed"] {
                let plan = serde_json::from_value(json!({
                    "entries": [
                        {"content":"First", "priority":"high", "status":status},
                        {"content":"Second", "priority":"low", "status":"in_progress"}
                    ]
                }))
                .unwrap();
                update(cx, id, SessionUpdate::Plan(plan))?;
            }
            chunk(cx, id, "m1", "task update")?;
        }
        "two-messages" => {
            chunk(cx, id, "m1", "first")?;
            let tool: ToolCall = serde_json::from_value(json!({
                "toolCallId": "t1",
                "title": "Terminal",
                "status": "pending",
            }))
            .unwrap();
            update(cx, id, SessionUpdate::ToolCall(tool))?;
            let rename: ToolCallUpdate = serde_json::from_value(json!({
                "toolCallId": "t1",
                "title": "Read notes.md",
            }))
            .unwrap();
            update(cx, id, SessionUpdate::ToolCallUpdate(rename))?;
            let done: ToolCallUpdate = serde_json::from_value(json!({
                "toolCallId": "t1",
                "status": "completed",
            }))
            .unwrap();
            update(cx, id, SessionUpdate::ToolCallUpdate(done))?;
            chunk(cx, id, "m2", "second")?;
        }
        "tool-activity" => {
            chunk(cx, id, "m1", "before")?;
            let call: ToolCall = serde_json::from_value(json!({
                "toolCallId":"edit", "title":"Edit a.rs", "kind":"edit",
                "status":"in_progress", "locations":[{"path":"a.rs"}],
                "content":[{"type":"diff","path":"a.rs","oldText":"x\n","newText":"y\n"}]
            }))
            .unwrap();
            update(cx, id, SessionUpdate::ToolCall(call))?;
            let done: ToolCallUpdate = serde_json::from_value(json!({
                "toolCallId":"edit", "status":"completed"
            }))
            .unwrap();
            update(cx, id, SessionUpdate::ToolCallUpdate(done))?;
            let placed: ToolCallUpdate = serde_json::from_value(json!({
                "toolCallId":"edit",
                "locations":[{"path":"a.rs","line":10},{"path":"a.rs","line":20}],
                "content":[
                    {"type":"diff","path":"a.rs","oldText":"x\n","newText":"x\ny\n"},
                    {"type":"diff","path":"a.rs","oldText":"q\n","newText":"r\n"}
                ]
            }))
            .unwrap();
            update(cx, id, SessionUpdate::ToolCallUpdate(placed))?;
            let read: ToolCall = serde_json::from_value(json!({
                "toolCallId":"read", "title":"Read a.rs", "kind":"read", "status":"completed",
                "rawInput":{"path":"a.rs","offset":10,"limit":5}
            }))
            .unwrap();
            update(cx, id, SessionUpdate::ToolCall(read))?;
            let failed: ToolCall = serde_json::from_value(json!({
                "toolCallId":"failed", "title":"Run false", "kind":"execute", "status":"failed",
                "content":[{"type":"content","content":{"type":"text","text":"Exit code 1"}}]
            }))
            .unwrap();
            update(cx, id, SessionUpdate::ToolCall(failed))?;
            chunk(cx, id, "m2", "after")?;
            let open: ToolCall = serde_json::from_value(json!({
                "toolCallId":"open", "title":"Run wait", "kind":"execute", "status":"in_progress"
            }))
            .unwrap();
            update(cx, id, SessionUpdate::ToolCall(open))?;
        }
        "check-env-one" | "check-env-two" => {
            let expected = if prompt == "check-env-one" {
                "synthetic-stored-one"
            } else {
                "synthetic-stored-two"
            };
            let matches = std::env::var("ANTHROPIC_AUTH_TOKEN").is_ok_and(|v| v == expected);
            chunk(cx, id, "m1", if matches { "true" } else { "false" })?;
        }
        "report" => {
            let bearer_hash = s
                .setup
                .bearer
                .as_deref()
                .map(shadows_core::testing::Token::hash_token);
            let text = json!({
                "cwd": s.cwd,
                "session": id,
                "how": s.how,
                "model": s.model,
                "effort": s.effort,
                "mode": s.mode,
                "config_dir": std::env::var("CLAUDE_CONFIG_DIR").ok(),
                "context_tokens": std::env::var("CLAUDE_CODE_MAX_CONTEXT_TOKENS").ok(),
                "compact_window": std::env::var("CLAUDE_CODE_AUTO_COMPACT_WINDOW").ok(),
                "has_token": std::env::var("ANTHROPIC_AUTH_TOKEN").is_ok_and(|v| !v.is_empty()),
                "claude": std::env::var("CLAUDE_CODE_EXECUTABLE")
                    .unwrap_or_default(),
                "mcp": s.setup.mcp,
                "bearer_hash": bearer_hash,
                "append": s.setup.append,
                "allowed": s.setup.allowed,
                "options": s.setup.options,
                "blocks": blocks,
                "resumes": resumes,
                "client_meta": client_meta,
            })
            .to_string();
            chunk(cx, id, "m1", &text)?;
        }
        "/context" => {
            if first_context {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            chunk(
                cx,
                id,
                "m1",
                concat!(
                    "| Category | Tokens | Percentage |\n",
                    "| Messages | 3.8k | 0.4% |\n",
                    "| System tools | 19.1k | 1.9% |\n",
                    "| Free space | 923.9k | 92.4% |",
                ),
            )?;
        }
        "activity-hold" => {
            let open: ToolCall = serde_json::from_value(json!({
                "toolCallId":"held", "title":"Run wait", "kind":"execute",
                "status":"in_progress"
            }))
            .unwrap();
            update(cx, id, SessionUpdate::ToolCall(open))?;
            let plan = serde_json::from_value(json!({
                "entries": [{"content":"Only", "priority":"high", "status":"in_progress"}]
            }))
            .unwrap();
            update(cx, id, SessionUpdate::Plan(plan))?;
            let mut cancelled = s.cancel.subscribe();
            while !*cancelled.borrow() {
                if cancelled.changed().await.is_err() {
                    break;
                }
            }
            return Ok((StopReason::Cancelled, None));
        }
        "hang" => {
            chunk(cx, id, "m1", "waiting")?;
            let mut cancelled = s.cancel.subscribe();
            while !*cancelled.borrow() {
                if cancelled.changed().await.is_err() {
                    break;
                }
            }
            return Ok((StopReason::Cancelled, None));
        }
        "steerable" => {
            chunk(cx, id, "m1", "waiting")?;
            let text = crate::steer::wait(s).await;
            // No pause: the reply may stream before Shadows has the answer,
            // as the real adapter's can.
            chunk(cx, id, "m2", &format!("steered: {text}"))?;
        }
        "wait-for-release" => {
            chunk(cx, id, "m1", "waiting")?;
            while !s.cwd.join("release").exists() {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        }
        "ignore-cancel" => {
            chunk(cx, id, "m1", "waiting")?;
            std::future::pending::<()>().await;
        }
        "exit" => {
            chunk(cx, id, "m1", "exiting")?;
            std::process::exit(3);
        }
        "ask-permission" => {
            let call: ToolCallUpdate = serde_json::from_value(json!({
                "toolCallId": "p1",
                "title": "Run echo probe",
            }))
            .unwrap();
            let req = RequestPermissionRequest::new(
                id.to_owned(),
                call,
                vec![
                    PermissionOption::new("allow_once", "Allow", PermissionOptionKind::AllowOnce),
                    PermissionOption::new(
                        "reject_once",
                        "Reject",
                        PermissionOptionKind::RejectOnce,
                    ),
                ],
            );
            let answer = cx.send_request(req).block_task().await?;
            let chosen = match answer.outcome {
                agent_client_protocol::schema::v1::RequestPermissionOutcome::Selected(x) => {
                    x.option_id.to_string()
                }
                _ => "cancelled".into(),
            };
            chunk(cx, id, "m1", &format!("permission: {chosen}"))?;
        }
        "turn-usage" | "turn-usage-second" | "turn-usage-refused" | "turn-usage-cancelled" => {
            let amount = if prompt == "turn-usage-second" {
                0.5
            } else {
                0.2
            };
            for used in [7, 999] {
                let u = serde_json::from_value(serde_json::json!({
                    "used":used,"size":200000,"cost":{"amount":amount,"currency":"USD"}
                }))
                .unwrap();
                update(cx, id, SessionUpdate::UsageUpdate(u))?;
            }
            if prompt == "turn-usage-refused" {
                return Ok((StopReason::Refusal, None));
            }
            if prompt == "turn-usage-cancelled" {
                return Ok((StopReason::Cancelled, None));
            }
            chunk(cx, id, "m1", "usage")?;
        }
        "usage" => {
            let u = UsageUpdate::new(1234, 200000).meta(
                json!({"_claude/model": "fake-large-answering"})
                    .as_object()
                    .unwrap()
                    .clone(),
            );
            update(cx, id, SessionUpdate::UsageUpdate(u))?;
            chunk(cx, id, "m1", "usage")?;
            let u = UsageUpdate::new(1234, 1000000).meta(
                json!({
                    "_claude/model": "fake-large-answering",
                    "_claude/rateLimit": {
                        "unifiedWindows": {
                            "five_hour": {
                                "utilization": 0.25,
                                "resetsAt": 1790212200,
                            },
                            "seven_day": {
                                "utilization": 0.5,
                                "resetsAt": 1790542800,
                            },
                        },
                    },
                })
                .as_object()
                .unwrap()
                .clone(),
            );
            update(cx, id, SessionUpdate::UsageUpdate(u))?;
        }
        "refuse" => {
            return Ok((StopReason::MaxTokens, None));
        }
        line if line.starts_with("mcp ") => {
            let text = call_mcp(&s.setup, line).await;
            chunk(cx, id, "m1", &text)?;
        }
        // As the real adapter does: the title is sent after the turn
        // has answered.
        line if line.starts_with("title ") => {
            chunk(cx, id, "m1", "titled")?;
            let title = line.strip_prefix("title ").unwrap_or_default();
            let info = SessionInfoUpdate::new().title(title.to_owned());
            return Ok((
                StopReason::EndTurn,
                Some(SessionUpdate::SessionInfoUpdate(info)),
            ));
        }
        _ => {
            chunk(cx, id, "m1", "hello ")?;
            chunk(cx, id, "m1", "from fake_acp")?;
        }
    }
    Ok((StopReason::EndTurn, None))
}
