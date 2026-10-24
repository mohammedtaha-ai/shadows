//! Compiled provider presets in product order.
use super::Providers;
use super::listing::Format;
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

fn all() -> Vec<ProviderPreset> {
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

impl Providers {
    pub fn provider_presets(&self) -> Vec<ProviderPreset> {
        all()
    }

    /// The one lookup of a preset by id.
    pub(super) fn preset(id: &str) -> Option<ProviderPreset> {
        all().into_iter().find(|p| p.id == id)
    }

    /// The preset whose URL is exactly `base_url`, which decides how it is listed.
    pub(super) fn preset_at(base_url: &str) -> Option<ProviderPreset> {
        all().into_iter().find(|p| p.base_url == base_url)
    }

    /// Where a preset lists its models, its format, and its key check, if any.
    pub(super) fn preset_listing(id: &str) -> (String, Format, Option<String>) {
        listing(id)
    }
}

fn listing(id: &str) -> (String, Format, Option<String>) {
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
