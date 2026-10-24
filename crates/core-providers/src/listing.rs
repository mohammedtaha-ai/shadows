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

impl Page {
    pub(super) fn parse(body: &[u8], format: Format) -> Result<Page, ()> {
        parse(body, format)
    }
}

fn parse(body: &[u8], format: Format) -> Result<Page, ()> {
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
#[path = "../tests/listing/invariants.rs"]
mod tests;
