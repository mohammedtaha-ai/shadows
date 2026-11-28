CREATE TABLE run_author_report (
    report_id TEXT PRIMARY KEY NOT NULL,
    attempt_id TEXT NOT NULL REFERENCES run_attempts(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('ended', 'conflict')),
    project_id TEXT NOT NULL,
    grant_id TEXT NOT NULL,
    workflow_id TEXT NOT NULL,
    task INTEGER NOT NULL,
    body TEXT NOT NULL,
    created_at TEXT NOT NULL,
    claimed_until TEXT NULL,
    acked_at TEXT NULL
);

CREATE INDEX run_author_report_grant ON run_author_report (grant_id, acked_at);
