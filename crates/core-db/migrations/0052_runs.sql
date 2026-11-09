-- One attempt is one executor session working one task of a frozen plan version.
-- `released_at` is set when its session can no longer write, which is when its
-- scope stops blocking an overlapping task. `command_id` and `fingerprint` make
-- Start idempotent. Ordering is by `number`, never by insertion.
CREATE TABLE run_attempts (
    id           TEXT PRIMARY KEY,
    project_id   TEXT NOT NULL,
    workflow_id  TEXT NOT NULL,
    version      INTEGER NOT NULL,
    task         INTEGER NOT NULL,
    number       INTEGER NOT NULL,
    thread_id    TEXT NOT NULL,
    operation_id TEXT NULL,
    model        TEXT NOT NULL,
    state        TEXT NOT NULL
        CHECK (state IN ('running','committed','stopped','interrupted','failed')),
    commit_sha   TEXT NULL,
    reason       TEXT NULL,
    writes_json  TEXT NOT NULL,
    started_at   TEXT NOT NULL,
    ended_at     TEXT NULL,
    released_at  TEXT NULL,
    command_id   TEXT NOT NULL UNIQUE,
    fingerprint  TEXT NOT NULL,
    UNIQUE (workflow_id, version, task, number),
    CHECK ((state = 'running') = (ended_at IS NULL))
);

CREATE INDEX idx_run_attempts_held ON run_attempts(project_id, released_at);

-- A person's mark that a task is done: what a `needs` link waits for.
CREATE TABLE run_task_marks (
    workflow_id TEXT NOT NULL,
    version     INTEGER NOT NULL,
    task        INTEGER NOT NULL,
    done        INTEGER NOT NULL CHECK (done IN (0,1)),
    done_at     TEXT NULL,
    command_id  TEXT NOT NULL UNIQUE,
    fingerprint TEXT NOT NULL,
    PRIMARY KEY (workflow_id, version, task)
);
