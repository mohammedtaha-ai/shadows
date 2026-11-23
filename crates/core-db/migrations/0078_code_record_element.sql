-- The design revisions pinned by each done task.
CREATE TABLE code_record_element (
    project_id TEXT NOT NULL,
    workflow_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    task INTEGER NOT NULL,
    element_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK (revision >= 1),
    action TEXT NOT NULL CHECK (action IN ('create', 'modify', 'delete')),
    path TEXT NOT NULL,
    symbol_id TEXT NULL,
    PRIMARY KEY (workflow_id, version, task, element_id)
);

CREATE INDEX code_record_element_project ON code_record_element (project_id, element_id);
