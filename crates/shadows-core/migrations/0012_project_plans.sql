-- Spec §16.9. A plan belongs to its project; a version to its plan, with its
-- writer and, from v2, its reason. One Draft per plan and an untouchable
-- frozen version are held here, not only in the store.
--
-- `workflow` is rebuilt: SQLite cannot drop a column a foreign key names.
-- Inside sqlx's transaction, `defer_foreign_keys` lets `DROP TABLE workflow`
-- leave `task`, `task_parent` and `draft_intent` pointing at nothing until
-- the new `workflow` holds the same ids again, before the commit checks.
PRAGMA defer_foreign_keys = ON;

CREATE TABLE plan (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES project(id) ON DELETE RESTRICT,
    state       TEXT NOT NULL CHECK (state IN ('Active','Archived')),
    created_at  TEXT NOT NULL,
    archived_at TEXT NULL,
    CHECK ((state = 'Archived') = (archived_at IS NOT NULL))
);

-- One plan per thread that has versions. Its id is the thread's: unique,
-- and stable if the migration is ever replayed on a copy.
INSERT INTO plan (id, project_id, state, created_at)
SELECT t.id, t.project_id, 'Active', MIN(w.created_at)
  FROM planning_thread t JOIN workflow w ON w.thread_id = t.id
 GROUP BY t.id, t.project_id;

CREATE TEMP TABLE workflow_before AS SELECT * FROM workflow;
DROP TABLE workflow;

CREATE TABLE workflow (
    id                   TEXT PRIMARY KEY,
    plan_id              TEXT NOT NULL REFERENCES plan(id) ON DELETE RESTRICT,
    state                TEXT NOT NULL CHECK (state IN ('Draft','Approved','Frozen','Running','Completed','Failed')),
    previous_version_id  TEXT NULL,
    source_plan_json     TEXT NULL,
    version              INTEGER NOT NULL CHECK (version >= 1),
    revision             INTEGER NOT NULL DEFAULT 0 CHECK (revision >= 0),
    title                TEXT NOT NULL,
    goal                 TEXT NOT NULL,
    change_reason        TEXT NULL CHECK (change_reason IS NULL OR trim(change_reason) <> ''),
    written_by_thread    TEXT NULL REFERENCES planning_thread(id) ON DELETE RESTRICT,
    written_by_operation TEXT NULL REFERENCES operation(id) ON DELETE RESTRICT,
    written_by_grant     TEXT NULL REFERENCES mcp_grant(id),
    created_at           TEXT NOT NULL,
    updated_at           TEXT NOT NULL,
    frozen_at            TEXT NULL,
    UNIQUE (id, plan_id),
    UNIQUE (previous_version_id),
    UNIQUE (plan_id, version),
    FOREIGN KEY (previous_version_id, plan_id) REFERENCES workflow(id, plan_id),
    CHECK (written_by_thread IS NOT NULL OR written_by_grant IS NOT NULL),
    CHECK (written_by_operation IS NULL OR written_by_thread IS NOT NULL),
    CHECK (state NOT IN ('Frozen','Running','Completed','Failed') OR frozen_at IS NOT NULL)
);

INSERT INTO workflow
       (id, plan_id, state, previous_version_id, source_plan_json, version, revision,
        title, goal, change_reason, written_by_thread, written_by_operation,
        written_by_grant, created_at, updated_at, frozen_at)
SELECT id, thread_id, state, previous_version_id, source_plan_json, version, revision,
       title, goal, NULL, thread_id, NULL, NULL, created_at, updated_at, frozen_at
  FROM workflow_before ORDER BY thread_id, version;
DROP TABLE workflow_before;

CREATE UNIQUE INDEX workflow_one_draft ON workflow(plan_id) WHERE state = 'Draft';

-- A frozen version, its tasks and its links never change (§13.2, §16.2).
-- Freezing is the update from Draft, so OLD.state is not yet 'Frozen'.
CREATE TRIGGER workflow_frozen_immutable BEFORE UPDATE ON workflow
WHEN OLD.state = 'Frozen'
BEGIN SELECT RAISE(ABORT, 'a frozen plan version never changes'); END;

CREATE TRIGGER task_frozen_insert BEFORE INSERT ON task
WHEN (SELECT state FROM workflow WHERE id = NEW.workflow_id) = 'Frozen'
BEGIN SELECT RAISE(ABORT, 'a frozen plan version never changes'); END;
CREATE TRIGGER task_frozen_update BEFORE UPDATE ON task
WHEN (SELECT state FROM workflow WHERE id = OLD.workflow_id) = 'Frozen'
BEGIN SELECT RAISE(ABORT, 'a frozen plan version never changes'); END;
CREATE TRIGGER task_frozen_delete BEFORE DELETE ON task
WHEN (SELECT state FROM workflow WHERE id = OLD.workflow_id) = 'Frozen'
BEGIN SELECT RAISE(ABORT, 'a frozen plan version never changes'); END;

CREATE TRIGGER task_parent_frozen_insert BEFORE INSERT ON task_parent
WHEN (SELECT state FROM workflow WHERE id = NEW.workflow_id) = 'Frozen'
BEGIN SELECT RAISE(ABORT, 'a frozen plan version never changes'); END;
CREATE TRIGGER task_parent_frozen_update BEFORE UPDATE ON task_parent
WHEN (SELECT state FROM workflow WHERE id = OLD.workflow_id) = 'Frozen'
BEGIN SELECT RAISE(ABORT, 'a frozen plan version never changes'); END;
CREATE TRIGGER task_parent_frozen_delete BEFORE DELETE ON task_parent
WHEN (SELECT state FROM workflow WHERE id = OLD.workflow_id) = 'Frozen'
BEGIN SELECT RAISE(ABORT, 'a frozen plan version never changes'); END;

-- §16.5: a deleted conversation stays a row, as a removed project does.
ALTER TABLE planning_thread ADD COLUMN removed_at TEXT NULL;
