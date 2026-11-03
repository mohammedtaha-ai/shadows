//! Groups of AYLUL agents and their conversation history.

mod bounds;
mod deliveries;
mod error;
mod groups;
mod mentions;
mod model;
mod prompt;
mod replies;
mod send;
mod store;

use core_agents::Agents;
use core_db::Storage;
use std::sync::Arc;

pub use error::MessagesError;
pub use model::{Address, Group, GroupId, Member, Message, MessageId, MessageKind, Speaker};
pub use replies::MessagesWorker;

pub struct Messages {
    storage: Arc<Storage>,
    agents: Arc<Agents>,
    wake: tokio::sync::Notify,
}

impl Messages {
    pub fn new(storage: Arc<Storage>, agents: Arc<Agents>) -> Arc<Self> {
        Arc::new(Self {
            storage,
            agents,
            wake: tokio::sync::Notify::new(),
        })
    }
}
