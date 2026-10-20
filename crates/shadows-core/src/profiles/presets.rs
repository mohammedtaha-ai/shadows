//! Compiled provider presets in product order.
use super::Profiles;
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PresetRegion {
    International,
    China,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct ProviderPreset {
    pub id: String,
    pub name: String,
    pub region: Option<PresetRegion>,
    pub base_url: String,
}

pub(super) fn all() -> Vec<ProviderPreset> {
    [
        (
            "minimax",
            "MiniMax",
            Some(PresetRegion::International),
            "https://api.minimax.io/anthropic",
        ),
        (
            "minimax-china",
            "MiniMax",
            Some(PresetRegion::China),
            "https://api.minimaxi.com/anthropic",
        ),
        ("anthropic", "Anthropic", None, "https://api.anthropic.com"),
        (
            "deepseek",
            "DeepSeek",
            None,
            "https://api.deepseek.com/anthropic",
        ),
        (
            "openrouter",
            "OpenRouter",
            None,
            "https://openrouter.ai/api",
        ),
        (
            "moonshot",
            "Moonshot (Kimi)",
            Some(PresetRegion::International),
            "https://api.moonshot.ai/anthropic",
        ),
        (
            "moonshot-china",
            "Moonshot (Kimi)",
            Some(PresetRegion::China),
            "https://api.moonshot.cn/anthropic",
        ),
    ]
    .into_iter()
    .map(|(id, name, region, base_url)| ProviderPreset {
        id: id.into(),
        name: name.into(),
        region,
        base_url: base_url.into(),
    })
    .collect()
}

impl Profiles {
    pub fn provider_presets(&self) -> Vec<ProviderPreset> {
        all()
    }
}

pub(super) fn listing(id: &str) -> (String, super::listing::Format, Option<String>) {
    use super::listing::Format;
    let (url, format) = match id {
        "minimax" => (
            "https://api.minimax.io/anthropic/v1/models?limit=100",
            Format::MiniMax,
        ),
        "minimax-china" => (
            "https://api.minimaxi.com/anthropic/v1/models?limit=100",
            Format::MiniMax,
        ),
        "anthropic" => (
            "https://api.anthropic.com/v1/models?limit=1000",
            Format::Anthropic,
        ),
        "deepseek" => ("https://api.deepseek.com/models", Format::DeepSeek),
        "openrouter" => ("https://openrouter.ai/api/v1/models", Format::OpenRouter),
        "moonshot" => ("https://api.moonshot.ai/v1/models", Format::Moonshot),
        "moonshot-china" => ("https://api.moonshot.cn/v1/models", Format::Moonshot),
        _ => unreachable!("compiled preset id"),
    };
    (
        url.into(),
        format,
        (id == "openrouter").then(|| "https://openrouter.ai/api/v1/key".into()),
    )
}
