CREATE TABLE run_author_report (
    report_id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('ended', 'conflict', 'reply', 'recheck')),
    source_id TEXT NOT NULL,
    attempt_id TEXT NULL REFERENCES run_attempts(id) ON DELETE CASCADE,
    project_id TEXT NOT NULL,
    grant_id TEXT NOT NULL,
    workflow_id TEXT NULL,
    task INTEGER NULL,
    body TEXT NOT NULL,
    created_at TEXT NOT NULL,
    claimed_until TEXT NULL,
    acked_at TEXT NULL
);

CREATE INDEX run_author_report_grant ON run_author_report (grant_id, acked_at);

-- The external agent that asked for a recheck batch (NULL when a Planner, the person or a
-- done mark began it); its report row goes to that grant.
ALTER TABLE run_rechecks ADD COLUMN asker_grant_id TEXT;
