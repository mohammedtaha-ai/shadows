-- The code record: what each done task's contract promised, file by file and
-- symbol by symbol, with what the code held of it when the task was marked done.
CREATE TABLE code_record_file (
    project_id TEXT NOT NULL,
    workflow_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    task INTEGER NOT NULL,
    path TEXT NOT NULL,
    action TEXT NOT NULL,
    purpose TEXT NOT NULL,
    commit_sha TEXT NULL,
    recorded_at TEXT NOT NULL,
    PRIMARY KEY (workflow_id, version, task, path)
);

CREATE INDEX code_record_file_project ON code_record_file (project_id);

CREATE TABLE code_record_symbol (
    project_id TEXT NOT NULL,
    workflow_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    task INTEGER NOT NULL,
    symbol_id TEXT NOT NULL,
    path TEXT NOT NULL,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    contract_json TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('found', 'missing', 'unchecked')),
    PRIMARY KEY (workflow_id, version, task, symbol_id)
);

CREATE INDEX code_record_symbol_project ON code_record_symbol (project_id);

CREATE TABLE code_record_unpromised (
    project_id TEXT NOT NULL,
    workflow_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    task INTEGER NOT NULL,
    path TEXT NOT NULL,
    name TEXT NOT NULL,
    PRIMARY KEY (workflow_id, version, task, path, name)
);

CREATE INDEX code_record_unpromised_project ON code_record_unpromised (project_id);
