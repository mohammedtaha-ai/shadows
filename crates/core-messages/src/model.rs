use core_db::newtype_id;
use core_threads::ThreadId;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

newtype_id! { GroupId }
newtype_id! { MessageId }

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Group {
    pub id: GroupId,
    pub name: String,
    pub members: Vec<Member>,
    pub needs_you: bool,
    pub needs_you_since: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct Member {
    pub slug: String,
    pub name: String,
    pub thread_id: ThreadId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Speaker {
    Person,
    Agent { slug: String, name: String },
    Shadows,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Address {
    Person,
    Agent { slug: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MessageKind {
    Chat,
    Notice,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Message {
    pub id: MessageId,
    pub group_id: GroupId,
    pub seq: i64,
    pub speaker: Speaker,
    pub to: Vec<Address>,
    pub kind: MessageKind,
    pub body: String,
    pub created_at: String,
}
