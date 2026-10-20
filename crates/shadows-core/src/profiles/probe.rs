//! Bounded outbound provider tests; keys and provider response text never enter diagnostics.
use super::{
    Profiles,
    listing::{self, Format},
    presets,
};
use crate::CoreError;
use std::time::Duration;

#[derive(Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct ProviderTest {
    pub preset: Option<String>,
    pub base_url: Option<String>,
    pub api_key: String,
}
#[derive(Clone, serde::Deserialize, utoipa::ToSchema)]
pub struct SavedProviderTest {
    pub api_key: Option<String>,
}

impl std::fmt::Debug for ProviderTest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ProviderTest { api_key: [REDACTED], .. }")
    }
}
impl std::fmt::Debug for SavedProviderTest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SavedProviderTest { api_key: [REDACTED] }")
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct ListedModel {
    pub id: String,
    pub context_window: Option<u32>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TestRefusal {
    KeyRefused,
    Unreachable,
    Unexpected,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct ProviderTestOutcome {
    pub ok: bool,
    pub models: Vec<ListedModel>,
    pub listed: bool,
    pub reason: Option<TestRefusal>,
    pub status: Option<u16>,
    pub message: Option<String>,
}
impl ProviderTestOutcome {
    fn refused(reason: TestRefusal, status: Option<u16>) -> Self {
        let message = match reason {
            TestRefusal::KeyRefused => "The provider refused this key.",
            TestRefusal::Unreachable => {
                "Could not reach the provider. Check your connection and retry."
            }
            TestRefusal::Unexpected => "The provider returned an unexpected response.",
        };
        Self {
            ok: false,
            models: vec![],
            listed: false,
            reason: Some(reason),
            status,
            message: Some(message.into()),
        }
    }
    fn success(models: Vec<ListedModel>, listed: bool, status: u16) -> Self {
        Self {
            ok: true,
            models,
            listed,
            reason: None,
            status: Some(status),
            message: None,
        }
    }
}
impl Profiles {
    pub async fn test_provider(
        &self,
        input: ProviderTest,
    ) -> Result<ProviderTestOutcome, CoreError> {
        let base_url = match (input.preset, input.base_url) {
            (Some(id), None) => {
                presets::all()
                    .into_iter()
                    .find(|p| p.id == id)
                    .ok_or_else(|| CoreError::ProfileInvalid("unknown provider preset".into()))?
                    .base_url
            }
            (None, Some(url)) => url.trim().to_owned(),
            _ => return Err(CoreError::ProfileInvalid("choose one preset or URL".into())),
        };
        super::providers::validate_url(&base_url)?;
        validate_key(&input.api_key)?;
        Ok(probe(&base_url, &input.api_key).await)
    }
}
pub(crate) fn validate_key(key: &str) -> Result<(), CoreError> {
    if key.trim().is_empty() {
        return Err(CoreError::ProfileInvalid("empty provider key".into()));
    }
    Ok(())
}

const LIMIT: usize = 8 * 1024 * 1024;

pub(crate) async fn probe(base_url: &str, api_key: &str) -> ProviderTestOutcome {
    match tokio::time::timeout(Duration::from_secs(10), run(base_url, api_key)).await {
        Ok(Ok(result)) => result,
        Ok(Err(result)) => result,
        Err(_) => ProviderTestOutcome::refused(TestRefusal::Unreachable, None),
    }
}

type ProbeResult<T> = Result<T, ProviderTestOutcome>;

async fn request(
    client: &reqwest::Client,
    url: &str,
    key: &str,
    format: Format,
    custom: bool,
) -> ProbeResult<(u16, Vec<u8>)> {
    let mut request = client.get(url);
    if custom || matches!(format, Format::Anthropic | Format::MiniMax) {
        let mut header = reqwest::header::HeaderValue::from_str(key)
            .map_err(|_| ProviderTestOutcome::refused(TestRefusal::Unexpected, None))?;
        header.set_sensitive(true);
        request = request
            .header("x-api-key", header)
            .header("anthropic-version", "2023-06-01");
    }
    if custom
        || matches!(
            format,
            Format::DeepSeek | Format::Moonshot | Format::OpenRouter
        )
    {
        request = request.bearer_auth(key);
    }
    let mut response = request
        .send()
        .await
        .map_err(|_| ProviderTestOutcome::refused(TestRefusal::Unreachable, None))?;
    let status = response.status().as_u16();
    let reason = match status {
        200 => None,
        404 if custom => return Ok((404, vec![])),
        401 | 403 => Some(TestRefusal::KeyRefused),
        500..=599 => Some(TestRefusal::Unreachable),
        _ => Some(TestRefusal::Unexpected),
    };
    if let Some(reason) = reason {
        return Err(ProviderTestOutcome::refused(reason, Some(status)));
    }
    if response
        .content_length()
        .is_some_and(|len| len > LIMIT as u64)
    {
        return Err(ProviderTestOutcome::refused(
            TestRefusal::Unexpected,
            Some(status),
        ));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| ProviderTestOutcome::refused(TestRefusal::Unreachable, Some(status)))?
    {
        if chunk.len() > LIMIT - bytes.len() {
            return Err(ProviderTestOutcome::refused(
                TestRefusal::Unexpected,
                Some(status),
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok((status, bytes))
}

async fn run(base_url: &str, key: &str) -> ProbeResult<ProviderTestOutcome> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|_| ProviderTestOutcome::refused(TestRefusal::Unreachable, None))?;
    let preset = presets::all().into_iter().find(|p| p.base_url == base_url);
    let config = preset.as_ref().map(|p| presets::listing(&p.id));
    let (url, format, check) = config.unwrap_or_else(|| {
        (
            format!("{}/v1/models", base_url.trim_end_matches('/')),
            Format::Anthropic,
            None,
        )
    });
    let custom = preset.is_none();
    if let Some(check) = check {
        let (status, body) = request(&client, &check, key, format, false).await?;
        let value: serde_json::Value = serde_json::from_slice(&body)
            .map_err(|_| ProviderTestOutcome::refused(TestRefusal::Unexpected, Some(status)))?;
        if !value.get("data").is_some_and(|v| v.is_object()) {
            return Err(ProviderTestOutcome::refused(
                TestRefusal::Unexpected,
                Some(status),
            ));
        }
    }
    let mut models = std::collections::BTreeMap::new();
    let mut next = reqwest::Url::parse(&url)
        .map_err(|_| ProviderTestOutcome::refused(TestRefusal::Unexpected, None))?;
    for _ in 0..5 {
        let (status, body) = request(&client, next.as_str(), key, format, custom).await?;
        if status == 404 {
            return Ok(ProviderTestOutcome::success(vec![], false, 404));
        }
        let page = listing::parse(&body, format)
            .map_err(|_| ProviderTestOutcome::refused(TestRefusal::Unexpected, Some(status)))?;
        for model in page.models {
            if model.id.contains(key) {
                return Err(ProviderTestOutcome::refused(
                    TestRefusal::Unexpected,
                    Some(status),
                ));
            }
            models.insert(model.id.clone(), model);
        }
        let Some(after) = page.after else {
            break;
        };
        next = reqwest::Url::parse(&url)
            .map_err(|_| ProviderTestOutcome::refused(TestRefusal::Unexpected, None))?;
        next.query_pairs_mut().append_pair("after_id", &after);
    }
    Ok(ProviderTestOutcome::success(
        models.into_values().collect(),
        true,
        200,
    ))
}
