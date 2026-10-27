//! The person's public record.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct Person {
    pub name: Option<String>,
    pub aylul_calls_me: Option<String>,
    pub language: Option<String>,
    pub theme: Theme,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct PersonPatch {
    #[serde(
        default,
        deserialize_with = "nullable_field",
        skip_serializing_if = "Option::is_none"
    )]
    pub name: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "nullable_field",
        skip_serializing_if = "Option::is_none"
    )]
    pub aylul_calls_me: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "nullable_field",
        skip_serializing_if = "Option::is_none"
    )]
    pub language: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "theme_field",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(nullable = false)]
    pub theme: Option<Theme>,
}

// A present null must survive deserialization so it can clear a stored value.
fn nullable_field<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Option<String>>, D::Error> {
    <Option<String> as serde::Deserialize>::deserialize(deserializer).map(Some)
}

// A missing theme keeps the stored value, but a present theme must be an enum value.
fn theme_field<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Theme>, D::Error> {
    <Theme as serde::Deserialize>::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Dark,
    Light,
    #[default]
    System,
}
