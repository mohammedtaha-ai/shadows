use std::path::PathBuf;

use serde_json::Value;

use super::{AgentHarness, AgentInvocation, StreamItem};
use crate::process::ProcessSpec;

pub struct ClaudeHarness {
    /// Spec §1.4: resolved from configuration, never from PATH.
    executable: PathBuf,
    /// Recorded per Operation. The stream contract belongs to one installation
    /// at one version, and this machine carries more than one.
    pub version: String,
}

impl ClaudeHarness {
    pub fn new(executable: PathBuf, version: String) -> Self {
        Self {
            executable,
            version,
        }
    }
}

impl AgentHarness for ClaudeHarness {
    fn to_process_spec(&self, inv: &AgentInvocation) -> ProcessSpec {
        let mut args: Vec<String> = vec![
            "--print".into(),
            "--output-format".into(),
            "stream-json".into(),
            "--verbose".into(),
            "--include-partial-messages".into(),
            "--model".into(),
            inv.model.clone(),
            "--permission-prompts".into(),
            "none".into(),
        ];
        match &inv.resume_session_id {
            Some(id) => {
                args.push("--resume".into());
                args.push(id.clone());
            }
            None => {
                args.push("--session-id".into());
                args.push(inv.session_id.clone());
            }
        }
        args.push(inv.prompt.clone());

        ProcessSpec {
            executable: self.executable.clone(),
            args,
            cwd: inv.cwd.clone(),
            env: Vec::new(),
            capture_stdout: true,
        }
    }

    fn classify(&self, line: &str) -> StreamItem {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            return StreamItem::Unparsed(line.to_string());
        };
        match v["type"].as_str() {
            Some("stream_event") => {
                let e = &v["event"];
                if e["type"] == "content_block_delta" && e["delta"]["type"] == "text_delta" {
                    return StreamItem::Delta {
                        text: e["delta"]["text"].as_str().unwrap_or("").to_string(),
                    };
                }
                StreamItem::Operational {
                    label: e["type"].as_str().unwrap_or("stream_event").to_string(),
                    session: None,
                }
            }
            Some(role @ ("assistant" | "user")) => StreamItem::Entry {
                uuid: v["uuid"].as_str().unwrap_or_default().to_string(),
                role: role.to_string(),
                text: render_content(&v["message"]["content"]),
            },
            Some("result") => StreamItem::TurnEnd {
                subtype: v["subtype"].as_str().unwrap_or("unknown").to_string(),
                stop_reason: v["stop_reason"].as_str().map(str::to_string),
            },
            Some("system") => StreamItem::Operational {
                label: format!("system/{}", v["subtype"].as_str().unwrap_or("?")),
                session: v["session_id"].as_str().map(str::to_string),
            },
            Some(other) => StreamItem::Operational {
                label: other.to_string(),
                session: None,
            },
            None => StreamItem::Unparsed(line.to_string()),
        }
    }
}

/// A durable line's content is a list of blocks. Flatten to the text a
/// ThreadEntry body holds; tool calls and results are named, not inlined.
fn render_content(content: &Value) -> String {
    let Some(blocks) = content.as_array() else {
        return String::new();
    };
    blocks
        .iter()
        .map(|b| match b["type"].as_str() {
            Some("text") => b["text"].as_str().unwrap_or("").to_string(),
            Some("thinking") => "[thinking]".to_string(),
            Some("tool_use") => format!("[tool_use: {}]", b["name"].as_str().unwrap_or("?")),
            Some("tool_result") => "[tool_result]".to_string(),
            Some(other) => format!("[{other}]"),
            None => String::new(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}
