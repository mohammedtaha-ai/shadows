-- An attempt's own or used task check: executor report, then Shadows' confirmation.
CREATE TABLE run_checks (
    attempt_id TEXT NOT NULL REFERENCES run_attempts(id) ON DELETE CASCADE,
    for_task INTEGER NOT NULL,
    check_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    run_json TEXT NOT NULL CHECK (json_valid(run_json)),
    folder TEXT NULL,
    reported TEXT NULL CHECK (reported IS NULL OR reported IN ('passed','failed','not_run')),
    reported_note TEXT NULL,
    outcome TEXT NULL CHECK (outcome IS NULL OR outcome IN
        ('passed','failed','timed_out','inconclusive','not_run')),
    exit_code INTEGER NULL,
    head_before TEXT NULL,
    head_after TEXT NULL,
    output TEXT NULL,
    truncated INTEGER NOT NULL DEFAULT 0 CHECK (truncated IN (0,1)),
    started_at TEXT NULL,
    finished_at TEXT NULL,
    PRIMARY KEY (attempt_id, for_task, check_id),
    CHECK ((outcome IS NULL) = (finished_at IS NULL))
);
