//! Provider input rules: presets, labels, URLs and the model list.
use super::{ProviderInput, Providers};
use crate::CoreError;

impl Providers {
    /// The input normalized, or the refusal: a preset supplies its name and URL.
    pub(crate) fn validated(&self, mut input: ProviderInput) -> Result<ProviderInput, CoreError> {
        input.name = input.name.take().map(|v| v.trim().to_owned());
        input.base_url = input.base_url.take().map(|v| v.trim().to_owned());
        if let Some(id) = &input.preset {
            let preset = Self::preset(id)
                .ok_or_else(|| CoreError::ProfileInvalid("unknown provider preset".into()))?;
            if input.name.as_ref().is_some_and(|v| v != &preset.name)
                || input
                    .base_url
                    .as_ref()
                    .is_some_and(|v| v != &preset.base_url)
            {
                return Err(CoreError::ProfileInvalid("preset values differ".into()));
            }
            input.name = Some(preset.name);
            input.base_url = Some(preset.base_url);
        }
        input.label = input
            .label
            .take()
            .map(|v| v.trim().to_owned())
            .filter(|v| !v.is_empty());
        if input
            .label
            .as_ref()
            .is_some_and(|v| v.chars().count() > 64 || v.chars().any(char::is_control))
        {
            return Err(CoreError::ProfileInvalid("invalid provider label".into()));
        }
        let url = input.base_url.as_deref().unwrap_or("");
        Self::validate_url(url)?;
        let mut models = std::collections::BTreeSet::new();
        let bad_models = !(1..=20).contains(&input.models.len())
            || input.models.iter_mut().any(|model| {
                let id = &model.id;
                let size = model.context_window.unwrap_or(200_000);
                model.context_window = Some(size);
                id == "default"
                    || !(1..=128).contains(&id.chars().count())
                    || id.chars().any(|c| c.is_whitespace() || c.is_control())
                    || !models.insert(id.clone())
                    || !(1000..=10_000_000).contains(&size)
            });
        if input.name.as_deref().unwrap_or("").is_empty()
            || input.name.as_ref().is_some_and(|v| v.len() > 256)
            || bad_models
        {
            return Err(CoreError::ProfileInvalid(
                "invalid provider configuration".into(),
            ));
        }
        Ok(input)
    }

    pub(super) fn validate_url(url: &str) -> Result<(), CoreError> {
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
            return Err(CoreError::ProfileInvalid("invalid provider URL".into()));
        }
        Ok(())
    }
}
