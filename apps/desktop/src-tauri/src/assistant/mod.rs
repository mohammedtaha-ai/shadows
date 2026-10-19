use reqwest::Method;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(
    tag = "action",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Request {
    Status {},
    List {},
    Create {
        command_id: String,
    },
    Snapshot {
        conversation_id: String,
    },
    Send {
        conversation_id: String,
        command_id: String,
        text: String,
    },
    Stop {
        conversation_id: String,
        turn_id: String,
    },
    Recover {
        conversation_id: String,
        turn_id: String,
    },
}
pub struct Operation {
    pub method: Method,
    pub path: String,
    pub body: Option<Value>,
}
impl Request {
    pub fn operation(&self) -> Result<Operation, &'static str> {
        let base = "/v1/nylra/assistant";
        let id = |value: &str| -> Result<(), &'static str> {
            if value.len() != 36
                || !value.bytes().enumerate().all(|(i, b)| {
                    if [8, 13, 18, 23].contains(&i) {
                        b == b'-'
                    } else {
                        b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
                    }
                })
            {
                return Err("request_refused");
            }
            Ok(())
        };
        let (method, path, body) = match self {
            Self::Status {} => (Method::GET, format!("{base}/status"), None),
            Self::List {} => (Method::GET, format!("{base}/conversations"), None),
            Self::Create { command_id } => {
                id(command_id)?;
                (
                    Method::POST,
                    format!("{base}/conversations"),
                    Some(json!({"commandId":command_id})),
                )
            }
            Self::Snapshot { conversation_id } => {
                id(conversation_id)?;
                (
                    Method::GET,
                    format!("{base}/conversations/{conversation_id}"),
                    None,
                )
            }
            Self::Send {
                conversation_id,
                command_id,
                text,
            } => {
                id(conversation_id)?;
                id(command_id)?;
                if text.trim().is_empty() || text.len() > 4096 {
                    return Err("request_refused");
                }
                (
                    Method::POST,
                    format!("{base}/conversations/{conversation_id}/turns"),
                    Some(json!({"commandId":command_id,"text":text})),
                )
            }
            Self::Stop {
                conversation_id,
                turn_id,
            }
            | Self::Recover {
                conversation_id,
                turn_id,
            } => {
                id(conversation_id)?;
                id(turn_id)?;
                let action = if matches!(self, Self::Stop { .. }) {
                    "stop"
                } else {
                    "recover"
                };
                (
                    Method::POST,
                    format!("{base}/conversations/{conversation_id}/turns/{turn_id}/{action}"),
                    Some(json!({})),
                )
            }
        };
        Ok(Operation { method, path, body })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn assistant_routes_are_fixed_and_reject_arbitrary_selectors() {
        let request: Request = serde_json::from_value(json!({"action":"send","conversationId":"00000000-0000-4000-8000-000000000001","commandId":"00000000-0000-4000-8000-000000000002","text":"Hello in English."})).unwrap();
        let op = request.operation().unwrap();
        assert_eq!(
            op.path,
            "/v1/nylra/assistant/conversations/00000000-0000-4000-8000-000000000001/turns"
        );
        assert_eq!(op.body.unwrap()["text"], "Hello in English.");
        assert!(serde_json::from_value::<Request>(
            json!({"action":"status","url":"https://elsewhere.test"})
        )
        .is_err());
    }
    #[test]
    fn assistant_path_escape_and_oversized_text_refuse() {
        let bad: Request =
            serde_json::from_value(json!({"action":"snapshot","conversationId":"../auth/session"}))
                .unwrap();
        assert!(bad.operation().is_err());
        let text = Request::Send {
            conversation_id: "00000000-0000-4000-8000-000000000001".into(),
            command_id: "00000000-0000-4000-8000-000000000002".into(),
            text: "x".repeat(4097),
        };
        assert!(text.operation().is_err());
    }
}
