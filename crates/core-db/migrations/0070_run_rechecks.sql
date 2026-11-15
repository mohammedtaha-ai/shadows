CREATE TABLE run_rechecks (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    plan_id TEXT NOT NULL,
    cause_workflow_id TEXT NOT NULL,
    cause_version INTEGER NOT NULL,
    cause_task INTEGER NOT NULL,
    cause_command TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    finished_at TEXT,
    notice_json TEXT CHECK (notice_json IS NULL OR json_valid(notice_json)),
    notice_sent_at TEXT
);
CREATE TABLE run_recheck_results (
    recheck_id TEXT NOT NULL REFERENCES run_rechecks(id) ON DELETE CASCADE,
    attempt_id TEXT NOT NULL REFERENCES run_attempts(id),
    workflow_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    task INTEGER NOT NULL,
    check_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    run_json TEXT NOT NULL CHECK (json_valid(run_json)),
    folder TEXT,
    outcome TEXT CHECK (outcome IN ('passed','failed','timed_out','inconclusive','not_run')),
    exit_code INTEGER,
    head TEXT,
    output TEXT,
    truncated INTEGER NOT NULL DEFAULT 0,
    started_at TEXT,
    finished_at TEXT,
    PRIMARY KEY (recheck_id, attempt_id, check_id)
);
