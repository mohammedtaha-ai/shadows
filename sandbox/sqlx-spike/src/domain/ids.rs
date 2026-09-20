//! ID types — all newtypes around Uuid.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

macro_rules! id_newtype {
    ($name:ident) => {
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            pub fn nil() -> Self {
                Self(Uuid::nil())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl From<Uuid> for $name {
            fn from(u: Uuid) -> Self {
                Self(u)
            }
        }
    };
}

id_newtype!(ProjectId);
id_newtype!(OperationId);
id_newtype!(EventId);
id_newtype!(CommandId);
id_newtype!(WorkflowId);
id_newtype!(TaskId);
id_newtype!(RuntimeInstanceId);
id_newtype!(ThreadId);
id_newtype!(ResearchId);
id_newtype!(Principal);

/// Timestamp alias for clarity. Always UTC.
pub type Timestamp = time::OffsetDateTime;

/// Helper: now in UTC.
pub fn now_utc() -> Timestamp {
    time::OffsetDateTime::now_utc()
}
