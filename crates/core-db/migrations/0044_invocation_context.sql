CREATE TABLE invocation_context (
    operation_id TEXT PRIMARY KEY REFERENCES operation(id),
    thread_id TEXT NOT NULL REFERENCES planning_thread(id),
    session_id TEXT NOT NULL,
    start_seq INTEGER NOT NULL,
    digest TEXT NOT NULL,
    text TEXT NOT NULL,
    sources_json TEXT NOT NULL,
    index_tokens INTEGER NOT NULL,
    detail_tokens INTEGER NOT NULL,
    over_tokens INTEGER NOT NULL,
    cuts_json TEXT NOT NULL,
    heard INTEGER NOT NULL DEFAULT 0 CHECK(heard IN (0,1)),
    created_at TEXT NOT NULL
);
CREATE INDEX invocation_context_session ON invocation_context(session_id,heard,start_seq);
