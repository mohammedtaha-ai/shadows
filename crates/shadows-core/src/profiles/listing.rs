//! Parse documented provider model listings without exposing response bodies.
use super::probe::ListedModel;

#[derive(Clone, Copy)]
pub(super) enum Format {
    Anthropic,
    MiniMax,
    DeepSeek,
    Moonshot,
    OpenRouter,
}

pub(super) struct Page {
    pub models: Vec<ListedModel>,
    pub after: Option<String>,
}

pub(super) fn parse(body: &[u8], format: Format) -> Result<Page, ()> {
    let value: serde_json::Value = serde_json::from_slice(body).map_err(|_| ())?;
    let rows = value.get("data").and_then(|v| v.as_array()).ok_or(())?;
    let mut models = Vec::new();
    for row in rows {
        let id = row
            .get("id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .ok_or(())?;
        if matches!(format, Format::OpenRouter)
            && !row
                .get("supported_parameters")
                .and_then(|v| v.as_array())
                .is_some_and(|v| v.iter().any(|v| v == "tools"))
        {
            continue;
        }
        let field = match format {
            Format::Anthropic => "max_input_tokens",
            Format::MiniMax => "",
            Format::DeepSeek => "context_window",
            Format::Moonshot | Format::OpenRouter => "context_length",
        };
        let context_window = match row.get(field) {
            None | Some(serde_json::Value::Null) => None,
            Some(v) => Some(u32::try_from(v.as_u64().ok_or(())?).map_err(|_| ())?),
        };
        models.push(ListedModel {
            id: id.into(),
            context_window,
        });
    }
    let paged = matches!(format, Format::Anthropic | Format::MiniMax);
    let more = match value.get("has_more") {
        None => false,
        Some(v) => v.as_bool().ok_or(())?,
    };
    let after = if paged && more {
        Some(
            value
                .get("last_id")
                .and_then(|v| v.as_str())
                .filter(|v| !v.is_empty())
                .ok_or(())?
                .to_owned(),
        )
    } else {
        None
    };
    Ok(Page { models, after })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn listing(format: Format, row: serde_json::Value) -> ListedModel {
        parse(
            &serde_json::to_vec(&serde_json::json!({"data":[row]})).unwrap(),
            format,
        )
        .unwrap()
        .models
        .remove(0)
    }
    #[test]
    fn anthropic_list_reads_max_input_tokens() {
        assert_eq!(
            listing(
                Format::Anthropic,
                serde_json::json!({"id":"claude",
            "max_input_tokens":200000})
            )
            .context_window,
            Some(200000)
        );
        let page = parse(
            br#"{"data":[],"has_more":true,"last_id":"next"}"#,
            Format::Anthropic,
        )
        .unwrap();
        assert_eq!(page.after.as_deref(), Some("next"));
        assert!(parse(br#"{"data":[{"id":2}]}"#, Format::Anthropic).is_err());
        assert!(
            parse(
                br#"{"data":[{"id":"m","max_input_tokens":"bad"}]}"#,
                Format::Anthropic
            )
            .is_err()
        );
    }
    #[test]
    fn minimax_list_has_no_windows() {
        assert_eq!(
            listing(
                Format::MiniMax,
                serde_json::json!({"id":"MiniMax-M3",
            "type":"model","created_at":"2026-10-21","context_window":1000000})
            )
            .context_window,
            None
        );
    }
    #[test]
    fn deepseek_list_reads_context_window() {
        assert_eq!(
            listing(
                Format::DeepSeek,
                serde_json::json!({"id":"deepseek-flash",
            "context_window":1000000})
            )
            .context_window,
            Some(1000000)
        );
    }
    #[test]
    fn moonshot_list_reads_context_length() {
        assert_eq!(
            listing(
                Format::Moonshot,
                serde_json::json!({"id":"kimi-k3",
            "context_length":1000000})
            )
            .context_window,
            Some(1000000)
        );
    }
    #[test]
    fn openrouter_list_keeps_tool_models() {
        let page = parse(
            br#"{"data":[{"id":"tools","context_length":123456,
            "supported_parameters":["tools","temperature"]},
            {"id":"plain","supported_parameters":["temperature"]}]}"#,
            Format::OpenRouter,
        )
        .unwrap();
        assert_eq!(page.models.len(), 1);
        assert_eq!(page.models[0].id, "tools");
        assert_eq!(page.models[0].context_window, Some(123456));
    }
}
