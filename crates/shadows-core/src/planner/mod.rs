//! Planner sessions: the adapter each thread holds, what it opens with, and
//! what it offers (§12.2–§12.4). A turn on one is `turns`' (§14.4).

mod context;
mod offers;
mod sessions;
mod settings;
mod setup;

pub use context::NoBreakdown;
pub use sessions::{LeaseError, OpenError, OpenSession, Sessions, SessionsConfig};
pub use settings::ModelRefused;
pub use setup::prompt_version;
