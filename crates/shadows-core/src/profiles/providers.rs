//! Provider validation and mutations.
use super::{Profiles, Provider, ProviderInput, store};
use crate::{CoreError, app::user_command};

fn validate(input: &mut ProviderInput) -> Result<(), CoreError> {
    input.name = input.name.take().map(|v| v.trim().to_owned());
    input.base_url = input.base_url.take().map(|v| v.trim().to_owned());
    if let Some(id) = &input.preset {
        let preset = super::presets::all()
            .into_iter()
            .find(|p| &p.id == id)
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
    validate_url(url)?;
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
    Ok(())
}

impl Profiles {
    pub async fn create_provider(
        &self,
        command_id: String,
        mut input: ProviderInput,
    ) -> Result<Provider, CoreError> {
        validate(&mut input)?;
        let command = user_command(command_id, "provider.create", serde_json::json!(input));
        let slug = store::save_provider(&self.storage, command, None, input).await?;
        self.provider(&slug).await
    }

    pub async fn update_provider(
        &self,
        command_id: String,
        slug: &str,
        mut input: ProviderInput,
    ) -> Result<Provider, CoreError> {
        validate(&mut input)?;
        let command = user_command(
            command_id,
            "provider.update",
            serde_json::json!({"slug": slug, "input": input}),
        );
        let slug = store::save_provider(&self.storage, command, Some(slug.into()), input).await?;
        self.provider(&slug).await
    }

    pub async fn delete_provider(&self, command_id: String, slug: &str) -> Result<(), CoreError> {
        let command = user_command(
            command_id,
            "provider.delete",
            serde_json::json!({"slug":slug}),
        );
        Ok(store::delete_provider(&self.storage, command, slug).await?)
    }
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
