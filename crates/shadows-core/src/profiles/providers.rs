//! Provider validation and mutations.
use super::{Profiles, Provider, ProviderInput, store};
use crate::{CoreError, app::user_command};

fn validate(input: &mut ProviderInput) -> Result<(), CoreError> {
    input.name = input.name.trim().to_owned();
    input.base_url = input.base_url.trim().to_owned();
    let url = &input.base_url;
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
    let mut models = std::collections::BTreeSet::new();
    let bad_models = !(1..=20).contains(&input.models.len())
        || input.models.iter().any(|id| {
            id == "default"
                || !(1..=128).contains(&id.chars().count())
                || id.chars().any(|c| c.is_whitespace() || c.is_control())
                || !models.insert(id)
        });
    let size = input.context_window.unwrap_or(200_000);
    if input.name.is_empty()
        || input.name.len() > 256
        || bad_url
        || bad_models
        || !(1000..=10_000_000).contains(&size)
    {
        return Err(CoreError::ProfileInvalid(
            "invalid provider configuration".into(),
        ));
    }
    input.context_window = Some(size);
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
