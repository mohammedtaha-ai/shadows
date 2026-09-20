CREATE TABLE durable_cursor (
    scope TEXT PRIMARY KEY,
    next_seq BIGINT NOT NULL
);

CREATE TABLE operation (
    id UUID PRIMARY KEY,
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    thread_id UUID,
    workflow_id UUID,
    task_id UUID,
    runtime_instance_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    durable_seq BIGINT NOT NULL,
    outcome JSONB
);

CREATE TABLE durable_event (
    id UUID PRIMARY KEY,
    scope TEXT NOT NULL,
    durable_seq BIGINT NOT NULL,
    kind TEXT NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    operation_id UUID,
    payload JSONB NOT NULL,
    UNIQUE (scope, durable_seq)
);

CREATE TABLE command_record (
    principal UUID NOT NULL,
    scope TEXT NOT NULL,
    command_id UUID NOT NULL,
    operation_id UUID NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (principal, scope, command_id)
);
