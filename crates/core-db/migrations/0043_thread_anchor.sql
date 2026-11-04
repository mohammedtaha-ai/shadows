CREATE TABLE design_thread_anchor (
    thread_id TEXT PRIMARY KEY REFERENCES planning_thread(id),
    kind TEXT NOT NULL CHECK(kind IN ('part','outcome')),
    item_id TEXT NOT NULL,
    set_at TEXT NOT NULL
);
