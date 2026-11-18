ALTER TABLE task ADD COLUMN copied_from TEXT REFERENCES task(id) ON DELETE SET NULL;

CREATE TABLE run_task_executors (
    task_id TEXT PRIMARY KEY NOT NULL REFERENCES task(id) ON DELETE CASCADE,
    plan_id TEXT NOT NULL,
    harness TEXT NOT NULL,
    model TEXT NOT NULL,
    effort TEXT,
    chosen_by TEXT NOT NULL CHECK (chosen_by IN ('person', 'planner')),
    chosen_at TEXT NOT NULL
);

CREATE TABLE run_executor_saves (
    id TEXT PRIMARY KEY NOT NULL,
    plan_id TEXT NOT NULL,
    task INTEGER NOT NULL,
    result_json TEXT NOT NULL CHECK (json_valid(result_json)),
    saved_at TEXT NOT NULL
);

ALTER TABLE run_attempts ADD COLUMN harness TEXT NOT NULL DEFAULT 'claude-code';
ALTER TABLE run_attempts ADD COLUMN effort TEXT;
