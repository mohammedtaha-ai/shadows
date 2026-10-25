//! UUID newtype declaration shared by service owners.

#[macro_export]
macro_rules! newtype_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
        )]
        pub struct $name(String);

        impl $name {
            pub fn generate() -> Self {
                Self(uuid::Uuid::new_v4().to_string())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub(crate) fn from_stored(id: String) -> Self {
                Self(id)
            }

            #[cfg(feature = "test-support")]
            pub fn from_literal(id: impl Into<String>) -> Self {
                Self(id.into())
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl ::utoipa::PartialSchema for $name {
            fn schema() -> ::utoipa::openapi::RefOr<::utoipa::openapi::schema::Schema> {
                ::utoipa::openapi::ObjectBuilder::new()
                    .schema_type(::utoipa::openapi::schema::Type::String)
                    .format(Some(::utoipa::openapi::SchemaFormat::KnownFormat(
                        ::utoipa::openapi::KnownFormat::Uuid,
                    )))
                    .into()
            }
        }

        impl ::utoipa::ToSchema for $name {}
    };
}
