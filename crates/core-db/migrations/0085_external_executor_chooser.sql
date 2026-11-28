CREATE TABLE run_task_executors_new (
    task_id TEXT PRIMARY KEY NOT NULL REFERENCES task(id) ON DELETE CASCADE,
    plan_id TEXT NOT NULL,
    harness TEXT NOT NULL,
    model TEXT NOT NULL,
    effort TEXT,
    chosen_by TEXT NOT NULL CHECK (chosen_by IN ('person', 'planner', 'external')),
    chosen_at TEXT NOT NULL
);

INSERT INTO run_task_executors_new
    (task_id, plan_id, harness, model, effort, chosen_by, chosen_at)
SELECT task_id, plan_id, harness, model, effort, chosen_by, chosen_at
FROM run_task_executors;

DROP TABLE run_task_executors;

ALTER TABLE run_task_executors_new RENAME TO run_task_executors;
