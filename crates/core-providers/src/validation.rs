//! Provider input rules: presets, labels, URLs and the model list.
use super::{ProviderInput, Providers, ProvidersError};

impl Providers {
    /// The input normalized, or the refusal: a preset supplies its name and URL.
    pub fn validated(&self, mut input: ProviderInput) -> Result<ProviderInput, ProvidersError> {
        input.name = input.name.take().map(|v| v.trim().to_owned());
        input.base_url = input.base_url.take().map(|v| v.trim().to_owned());
        if let Some(id) = &input.preset {
            let preset = Self::preset(id)
                .ok_or_else(|| ProvidersError::InvalidProfile("unknown provider preset".into()))?;
            if input.name.as_ref().is_some_and(|v| v != &preset.name) {
                return Err(invalid("name", "Name must match the preset."));
            }
            if input
                .base_url
                .as_ref()
                .is_some_and(|v| v != &preset.base_url)
            {
                return Err(invalid("base_url", "Base URL must match the preset."));
            }
            input.name = Some(preset.name);
            input.base_url = Some(preset.base_url);
        }
        input.label = input
            .label
            .take()
            .map(|v| v.trim().to_owned())
            .filter(|v| !v.is_empty());
        if input.label.as_ref().is_some_and(|v| v.chars().count() > 64) {
            return Err(invalid("label", "Label must be at most 64 characters."));
        }
        if input
            .label
            .as_ref()
            .is_some_and(|v| v.chars().any(char::is_control))
        {
            return Err(invalid(
                "label",
                "Label must not contain control characters.",
            ));
        }
        let url = input.base_url.as_deref().unwrap_or("");
        Self::validate_url(url).map_err(|_| invalid("base_url", "Base URL must use HTTPS or loopback HTTP, with a host, no credentials, and no invalid authority or whitespace."))?;
        if input.name.as_deref().unwrap_or("").is_empty()
            || input.name.as_ref().is_some_and(|v| v.len() > 256)
        {
            return Err(invalid("name", "Name must be 1 to 256 bytes."));
        }
        if !(1..=20).contains(&input.models.len()) {
            return Err(invalid("models", "Models must be 1 to 20 ids."));
        }
        let mut models = std::collections::BTreeSet::new();
        for (index, model) in input.models.iter_mut().enumerate() {
            let id = &model.id;
            let field = format!("models[{index}].id");
            if id == "default" {
                return Err(invalid(&field, "default is reserved."));
            }
            if !(1..=128).contains(&id.chars().count())
                || id
                    .chars()
                    .any(|c| c.is_whitespace() || c.is_control() || c == '\u{feff}')
            {
                return Err(invalid(
                    &field,
                    "Model ids must be 1 to 128 printable characters with no spaces.",
                ));
            }
            if !models.insert(id.clone()) {
                return Err(invalid(&field, "Each model is listed once."));
            }
            let size = model.context_window.unwrap_or(200_000);
            if !(1000..=10_000_000).contains(&size) {
                return Err(invalid(
                    &format!("models[{index}].context_window"),
                    "Context window must be from 1000 to 10000000.",
                ));
            }
            model.context_window = Some(size);
        }
        Ok(input)
    }

    pub(super) fn validate_url(url: &str) -> Result<(), ProvidersError> {
        let (scheme, rest) = url.split_once("://").unwrap_or(("", ""));
        let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
        let host = if authority.starts_with('[') {
            authority
                .find(']')
                .map(|end| &authority[..=end])
                .unwrap_or("")
        } else {
            authority.split(':').next().unwrap_or("")
        };
        let secure = scheme == "https"
            || (scheme == "http" && matches!(host, "localhost" | "127.0.0.1" | "[::1]"));
        let bad_url = !secure
            || host.is_empty()
            || authority.contains(['@', '%', '\\'])
            || url.chars().any(|c| c.is_whitespace() || c.is_control());
        if bad_url {
            return Err(ProvidersError::InvalidProfile(
                "invalid provider URL".into(),
            ));
        }
        Ok(())
    }
}

fn invalid(field: &str, message: &str) -> ProvidersError {
    ProvidersError::InvalidField {
        field: field.into(),
        message: message.into(),
    }
}
