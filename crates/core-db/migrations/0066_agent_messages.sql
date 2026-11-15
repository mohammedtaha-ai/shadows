-- Plain questions to an executor, ordered independently of attempts and additions.
CREATE TABLE run_agent_messages (
    id TEXT PRIMARY KEY,
    attempt_id TEXT NOT NULL REFERENCES run_attempts(id) ON DELETE CASCADE,
    thread_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    message TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('waiting','sent','replied','delivered','failed')),
    operation_id TEXT NULL,
    reply TEXT NULL,
    reason TEXT NULL,
    created_at TEXT NOT NULL,
    answered_at TEXT NULL,
    delivered_at TEXT NULL,
    notice_json TEXT NULL CHECK (notice_json IS NULL OR json_valid(notice_json)),
    UNIQUE (thread_id, ordinal)
);
