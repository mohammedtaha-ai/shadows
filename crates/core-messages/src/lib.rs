//! Groups of AYLUL agents and their conversation history.

mod error;
mod groups;
mod model;
mod store;

use core_agents::Agents;
use core_db::Storage;
use std::sync::Arc;

pub use error::MessagesError;
pub use model::{Address, Group, GroupId, Member, Message, MessageId, MessageKind, Speaker};

pub struct Messages {
    storage: Arc<Storage>,
    agents: Arc<Agents>,
}

impl Messages {
    pub fn new(storage: Arc<Storage>, agents: Arc<Agents>) -> Arc<Self> {
        Arc::new(Self { storage, agents })
    }
}
