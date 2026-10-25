CREATE TABLE harness_thread_mode (
    thread_id TEXT PRIMARY KEY REFERENCES planning_thread(id),
    harness_kind TEXT NOT NULL,
    mode TEXT NOT NULL
);
