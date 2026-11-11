-- One row is the record a task left when it stopped on the cap of additions, keyed by the
-- attempt that was stopped. `record_json` holds the chain of attempts, the commands the
-- executor ran and, once the Planner writes them, what stopped the work and what the plan
-- should have said.
CREATE TABLE run_lessons (
    attempt_id  TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL,
    workflow_id TEXT NOT NULL,
    version     INTEGER NOT NULL,
    task        INTEGER NOT NULL,
    record_json TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE INDEX idx_run_lessons_task ON run_lessons(workflow_id, version, task);
