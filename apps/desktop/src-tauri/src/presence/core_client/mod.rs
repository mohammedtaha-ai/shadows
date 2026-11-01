//! Every request the companion makes to Core. Binding validation and submission
//! run on command threads and stay blocking; observation runs on the watcher's
//! runtime and stays async. Nothing else in the companion builds a Core request.
mod blocking;
mod observer;
pub use blocking::Blocking;
pub use observer::{Client, Feed, Problem};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct RuntimeIdentity {
    pub runtime_id: String,
    pub configuration_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct PermissionOption {
    pub option_id: String,
    pub name: String,
    pub kind: String,
}

/// One request Core holds for the person, oldest first in the list that carries it.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct WaitingPermission {
    pub thread_id: String,
    #[serde(default)]
    pub agent: String,
    pub request_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub options: Vec<PermissionOption>,
}
