#[derive(Debug, Clone, serde::Serialize)]
pub struct PlanningThread {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ThreadEntry {
    pub id: String,
    pub thread_id: String,
    pub ordinal: i64,
    pub kind: String,
    pub author_kind: String,
    pub author_id: String,
    pub body: String,
    pub refs: Vec<EntryRef>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EntryRef {
    Decision(String),
    Research(String),
    Workflow(String),
    Operation(String),
}
