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
