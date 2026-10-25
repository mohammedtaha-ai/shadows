//! Every request the companion makes to Core. Binding validation and submission
//! run on command threads and stay blocking; observation runs on the watcher's
//! runtime and stays async. Nothing else in the companion builds a Core request.
mod blocking;
mod observer;
pub use blocking::Blocking;
pub use observer::{Client, Problem};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct RuntimeIdentity {
    pub runtime_id: String,
    pub configuration_id: String,
}
