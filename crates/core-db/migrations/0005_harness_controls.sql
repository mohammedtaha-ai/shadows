-- Harness controls (spec §12.4-§12.9).
--
-- A thread runs on one harness, locked from its first operation (§12.6); a
-- fork names its source, the entry it was taken at and the source's harness
-- session, all three or none, written once (§12.9). Every entry a turn writes
-- names that turn (§12.7); entries written before this migration keep NULL,
-- because which turn wrote them could only be inferred from timing.
ALTER TABLE planning_thread ADD COLUMN harness_kind TEXT NOT NULL DEFAULT 'claude-code';
ALTER TABLE planning_thread ADD COLUMN forked_from_thread TEXT NULL REFERENCES planning_thread(id) ON DELETE RESTRICT;
ALTER TABLE planning_thread ADD COLUMN forked_from_entry TEXT NULL REFERENCES thread_entry(id) ON DELETE RESTRICT;
ALTER TABLE planning_thread ADD COLUMN fork_session_id TEXT NULL;

CREATE TRIGGER planning_thread_fork_all_or_none
BEFORE INSERT ON planning_thread
WHEN NOT ((NEW.forked_from_thread IS NULL AND NEW.forked_from_entry IS NULL AND NEW.fork_session_id IS NULL)
       OR (NEW.forked_from_thread IS NOT NULL AND NEW.forked_from_entry IS NOT NULL AND NEW.fork_session_id IS NOT NULL))
BEGIN SELECT RAISE(ABORT, 'fork_columns_all_or_none'); END;

CREATE TRIGGER planning_thread_fork_immutable
BEFORE UPDATE OF forked_from_thread, forked_from_entry, fork_session_id ON planning_thread
BEGIN SELECT RAISE(ABORT, 'fork_columns_immutable'); END;

CREATE TRIGGER planning_thread_harness_locked
BEFORE UPDATE OF harness_kind ON planning_thread
WHEN NEW.harness_kind <> OLD.harness_kind
 AND EXISTS (SELECT 1 FROM operation WHERE thread_id = OLD.id)
BEGIN SELECT RAISE(ABORT, 'harness_locked'); END;

ALTER TABLE thread_entry ADD COLUMN operation_id TEXT NULL REFERENCES operation(id) ON DELETE RESTRICT;

-- The modes each project allows per harness (§12.5): a set, keyed by all
-- three columns, never an ordering. Existing projects get the policy's modes.
CREATE TABLE project_mode (
    project_id   TEXT NOT NULL REFERENCES project(id) ON DELETE RESTRICT,
    harness_kind TEXT NOT NULL,
    mode_id      TEXT NOT NULL,
    PRIMARY KEY (project_id, harness_kind, mode_id)
);
INSERT INTO project_mode (project_id, harness_kind, mode_id)
    SELECT id, 'claude-code', 'acceptEdits' FROM project
    UNION ALL SELECT id, 'claude-code', 'auto' FROM project;

-- What a turn asked of the harness and what the harness reported (§12.7).
-- The requested, path and version columns are frozen by the trigger below;
-- the observed ones are written once, with the turn's terminal transition.
CREATE TABLE agent_invocation (
    id                TEXT PRIMARY KEY,
    operation_id      TEXT NOT NULL UNIQUE REFERENCES operation(id) ON DELETE RESTRICT,
    role              TEXT NOT NULL,
    harness_kind      TEXT NOT NULL,
    harness_path      TEXT NOT NULL,
    harness_version   TEXT NOT NULL,
    agent_path        TEXT NOT NULL,
    agent_version     TEXT NOT NULL,
    requested_model   TEXT NOT NULL,
    requested_mode    TEXT NOT NULL,
    requested_effort  TEXT NULL,
    profile_json      TEXT NOT NULL DEFAULT '{}',
    native_session_id TEXT NULL,
    observed_model    TEXT NULL,
    context_used      INTEGER NULL,
    context_window    INTEGER NULL,
    created_at        TEXT NOT NULL
);
CREATE INDEX agent_invocation_by_operation ON agent_invocation(operation_id, created_at, id);

CREATE TRIGGER agent_invocation_requested_immutable
BEFORE UPDATE OF requested_model, requested_mode, requested_effort,
                 harness_path, harness_version, agent_path, agent_version ON agent_invocation
BEGIN SELECT RAISE(ABORT, 'invocation_requested_immutable'); END;

-- Per harness, the model and effort of the last turn started (§12.4). Not
-- journal data: one row per harness, latest wins.
CREATE TABLE harness_preference (
    harness_kind TEXT PRIMARY KEY,
    model        TEXT NOT NULL,
    effort       TEXT NULL,
    updated_at   TEXT NOT NULL
);

-- Per harness, the account limits last reported (§12.8). Latest wins.
CREATE TABLE harness_limit (
    harness_kind          TEXT PRIMARY KEY,
    five_hour_utilization REAL NULL,
    five_hour_resets_at   INTEGER NULL,
    seven_day_utilization REAL NULL,
    seven_day_resets_at   INTEGER NULL,
    observed_at           TEXT NOT NULL
);
