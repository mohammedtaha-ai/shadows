CREATE TABLE project (
    id                  TEXT PRIMARY KEY,
    slug                TEXT NOT NULL UNIQUE,
    name                TEXT NOT NULL,
    default_config_ref  TEXT NULL,
    created_at          TEXT NOT NULL
);

CREATE TABLE planning_thread (
    id                  TEXT PRIMARY KEY,
    project_id          TEXT NOT NULL REFERENCES project(id) ON DELETE RESTRICT,
    title               TEXT NOT NULL,
    status              TEXT NOT NULL CHECK (status IN ('Open','Closed')),
    next_entry_ordinal  INTEGER NOT NULL DEFAULT 1 CHECK (next_entry_ordinal > 0),
    created_at          TEXT NOT NULL
);
CREATE INDEX idx_thread_by_project ON planning_thread(project_id, created_at, id);

CREATE TABLE thread_entry (
    id           TEXT PRIMARY KEY,
    thread_id    TEXT NOT NULL REFERENCES planning_thread(id) ON DELETE RESTRICT,
    ordinal      INTEGER NOT NULL CHECK (ordinal > 0),
    kind         TEXT NOT NULL,
    author_kind  TEXT NOT NULL,
    author_id    TEXT NOT NULL,
    body         TEXT NOT NULL,
    refs_json    TEXT NOT NULL DEFAULT '[]',
    created_at   TEXT NOT NULL,
    UNIQUE (thread_id, ordinal)
);

CREATE TABLE runtime_instance (
    id          TEXT PRIMARY KEY,
    version     TEXT NOT NULL,
    started_at  TEXT NOT NULL,
    stopped_at  TEXT NULL,
    stop_kind   TEXT NULL CHECK (stop_kind IS NULL OR stop_kind IN ('Graceful','Escalated')),
    CHECK ((stopped_at IS NULL) = (stop_kind IS NULL))
);

CREATE TABLE operation (
    id                        TEXT PRIMARY KEY,
    kind                      TEXT NOT NULL CHECK (kind IN ('PlannerTurn')),
    status_kind               TEXT NOT NULL DEFAULT 'Pending'
        CHECK (status_kind IN ('Pending','Running','Completed','Failed','Cancelled','Interrupted')),
    thread_id                 TEXT NULL REFERENCES planning_thread(id) ON DELETE RESTRICT,
    runtime_instance_id       TEXT NOT NULL REFERENCES runtime_instance(id) ON DELETE RESTRICT,
    outcome_json              TEXT NULL,
    failure_stage             TEXT NULL,
    failure_reason            TEXT NULL,
    interrupt_reason          TEXT NULL,
    cancel_requested_at       TEXT NULL,
    cancel_requested_by_kind  TEXT NULL,
    cancel_requested_by_id    TEXT NULL,
    created_at                TEXT NOT NULL,
    started_at                TEXT NULL,
    finished_at               TEXT NULL,

    -- Spec §6.14 state-shape constraints, one CHECK per status.
    CHECK (status_kind <> 'Pending' OR (
        started_at IS NULL AND finished_at IS NULL AND outcome_json IS NULL
        AND failure_stage IS NULL AND failure_reason IS NULL AND interrupt_reason IS NULL)),
    CHECK (status_kind <> 'Running' OR (
        started_at IS NOT NULL AND finished_at IS NULL AND outcome_json IS NULL
        AND failure_stage IS NULL AND failure_reason IS NULL AND interrupt_reason IS NULL)),
    CHECK (status_kind <> 'Completed' OR (
        started_at IS NOT NULL AND finished_at IS NOT NULL AND outcome_json IS NOT NULL
        AND failure_stage IS NULL AND failure_reason IS NULL AND interrupt_reason IS NULL)),
    CHECK (status_kind <> 'Failed' OR (
        finished_at IS NOT NULL AND failure_stage IS NOT NULL AND failure_reason IS NOT NULL
        AND outcome_json IS NULL AND interrupt_reason IS NULL)),
    CHECK (status_kind <> 'Cancelled' OR (
        finished_at IS NOT NULL AND outcome_json IS NULL
        AND failure_stage IS NULL AND interrupt_reason IS NULL)),
    CHECK (status_kind <> 'Interrupted' OR (
        finished_at IS NOT NULL AND outcome_json IS NULL
        AND failure_stage IS NULL AND interrupt_reason IS NOT NULL)),

    -- Spec §6.14: the three cancellation columns are all NULL or all NOT NULL.
    CHECK ((cancel_requested_at IS NULL) = (cancel_requested_by_kind IS NULL)),
    CHECK ((cancel_requested_at IS NULL) = (cancel_requested_by_id IS NULL))
);
CREATE INDEX idx_operation_by_thread ON operation(thread_id, created_at, id);
CREATE INDEX idx_operation_non_terminal
    ON operation(runtime_instance_id)
    WHERE status_kind IN ('Pending','Running');

CREATE TABLE durable_event (
    seq             INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id        TEXT NOT NULL UNIQUE,
    kind            TEXT NOT NULL,
    project_id      TEXT NULL REFERENCES project(id) ON DELETE RESTRICT,
    thread_id       TEXT NULL REFERENCES planning_thread(id) ON DELETE RESTRICT,
    operation_id    TEXT NULL REFERENCES operation(id) ON DELETE RESTRICT,
    actor_kind      TEXT NULL,
    actor_id        TEXT NULL,
    causation_kind  TEXT NULL,
    causation_ref   TEXT NULL,
    correlation_id  TEXT NULL,
    payload_json    TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    CHECK ((actor_kind IS NULL) = (actor_id IS NULL)),
    CHECK ((causation_kind IS NULL) = (causation_ref IS NULL))
);
CREATE INDEX idx_event_project   ON durable_event(project_id, seq);
CREATE INDEX idx_event_thread    ON durable_event(thread_id, seq);
CREATE INDEX idx_event_operation ON durable_event(operation_id, seq);

CREATE TABLE command_record (
    principal_kind       TEXT NOT NULL,
    principal_id         TEXT NOT NULL,
    command_scope_kind   TEXT NOT NULL,
    command_scope_key    TEXT NOT NULL,
    command_id           TEXT NOT NULL,
    command_kind         TEXT NOT NULL,
    command_schema_ver   INTEGER NOT NULL CHECK (command_schema_ver > 0),
    request_fingerprint  TEXT NOT NULL,
    outcome_kind         TEXT NOT NULL CHECK (outcome_kind IN ('Entity','NoContent')),
    entity_kind          TEXT NULL,
    outcome_ref          TEXT NULL,
    recorded_at          TEXT NOT NULL,
    PRIMARY KEY (principal_kind, principal_id, command_scope_kind, command_scope_key, command_id),
    CHECK (outcome_kind <> 'NoContent' OR (entity_kind IS NULL AND outcome_ref IS NULL)),
    CHECK (outcome_kind <> 'Entity'    OR (entity_kind IS NOT NULL AND outcome_ref IS NOT NULL))
);
