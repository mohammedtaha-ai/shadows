CREATE TABLE context_forget (
    seq INTEGER PRIMARY KEY,
    thread_id TEXT NOT NULL REFERENCES planning_thread(id),
    operation_id TEXT REFERENCES operation(id),
    reason TEXT NOT NULL CHECK(reason IN ('compaction','request')),
    trigger TEXT,
    created_at TEXT NOT NULL
);
CREATE INDEX context_forget_thread ON context_forget(thread_id,seq);
