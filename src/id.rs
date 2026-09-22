//! One job: the UUID id newtype pattern, defined once.
//!
//! Spec §4.1 requires domain ids to be UUID newtypes rather than `String`, so
//! that the compiler tells two ids of different kinds apart. Milestone 0 now
//! carries five of them. Five hand-written copies of the same twenty-five lines
//! is how one of them quietly acquires a `From<String>`, a `Deref`, or an
//! `AsRef<str>` — any of which reopens from the back door exactly the hazard the
//! newtypes exist to close, and only in that one type, where nobody is looking.
//!
//! So the shape is declared once here and each id is created by its own domain
//! module. This module knows nothing about projects, threads or operations; it
//! is the pattern, not a home for types that belong elsewhere.
//!
//! **Deliberately absent from the generated impls:**
//!
//! - `From<String>` / `From<&str>`: would make every id interchangeable with
//!   every other by way of a string, which is the hazard.
//! - `Deref<Target = str>` / `AsRef<str>`: would let an id be passed wherever a
//!   `&str` is expected, silently restoring the swap at any such call site.
//! - `Default`: a `default()` that mints a fresh random UUID is a trap — it
//!   reads as a cheap empty value while producing a different id on every call.
//!   The generator is named `generate` rather than `new` so that
//!   `clippy::new_without_default` does not ask for one.
//!
//! `Deserialize` *is* derived, and is the one way outside code can build an id
//! from arbitrary text. That is not a hole in the above: it is needed to read
//! stored values back (`EntryRef` round-trips through `refs_json`), and it grants
//! no ability to pass one kind of id where another is expected, which is what
//! these types are for. They distinguish ids; they do not validate them.

macro_rules! newtype_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
        )]
        pub struct $name(String);

        impl $name {
            /// Mints a fresh v4 UUID. Named `generate` and not `new`: see the
            /// module docs on `Default`.
            pub fn generate() -> Self {
                Self(uuid::Uuid::new_v4().to_string())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Reconstructs an id already known to be valid — a value read back
            /// from storage. Storage is the only caller; this is not a parser,
            /// and it is deliberately not `pub`.
            pub(crate) fn from_stored(id: String) -> Self {
                Self(id)
            }

            /// Builds an id from a literal, for tests that seed rows with known
            /// values and then assert on them. Gated behind `test-support`, which
            /// `cargo test` enables through the self dev-dependency and
            /// `cargo build` never does — so product code cannot reach it, and
            /// the rule that an id is either generated or read back from storage
            /// still holds everywhere that ships.
            #[cfg(feature = "test-support")]
            pub fn from_literal(id: impl Into<String>) -> Self {
                Self(id.into())
            }
        }

        /// Needed by `tracing`'s `%field` syntax, which every recovery and
        /// transition log uses. It was briefly removed as unused surface; it was
        /// unused only because the code that logs these ids had not been typed
        /// yet. `Display` does not weaken the type — unlike `Deref` or
        /// `AsRef<str>`, it cannot be applied implicitly where a `&str` is
        /// expected, so it prints an id without ever standing in for one.
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

pub(crate) use newtype_id;
