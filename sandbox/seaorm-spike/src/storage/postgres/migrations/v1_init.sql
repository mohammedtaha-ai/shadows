CREATE TABLE IF NOT EXISTS project (
    id UUID PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);
CREATE TABLE IF NOT EXISTS operation (
    id UUID PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    thread_id UUID,
    workflow_id UUID,
    task_id UUID,
    runtime_instance_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    durable_seq BIGINT NOT NULL,
    outcome TEXT
);
CREATE TABLE IF NOT EXISTS durable_event (
    id UUID PRIMARY KEY NOT NULL,
    durable_seq BIGINT NOT NULL,
    kind TEXT NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    operation_id UUID,
    payload JSONB NOT NULL
);
CREATE TABLE IF NOT EXISTS command_record (
    command_id UUID PRIMARY KEY NOT NULL,
    principal UUID NOT NULL,
    scope TEXT NOT NULL,
    operation_id UUID NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL
);
CREATE TABLE IF NOT EXISTS research_artifact (
    id UUID PRIMARY KEY NOT NULL,
    project_id UUID NOT NULL,
    title TEXT NOT NULL,
    source TEXT,
    summary TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    tsv tsvector
);
CREATE TABLE IF NOT EXISTS durable_seq_counter (n BIGINT PRIMARY KEY);
INSERT INTO durable_seq_counter(n) VALUES (0) ON CONFLICT DO NOTHING;
CREATE TABLE IF NOT EXISTS workflow_status (
    workflow_id UUID PRIMARY KEY NOT NULL,
    status TEXT NOT NULL,
    set_at TIMESTAMPTZ NOT NULL
);
