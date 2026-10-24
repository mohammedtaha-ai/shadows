-- §23.2: immutable project additions and invocation standards provenance.
CREATE TABLE standards_additions_version (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES project(id) ON DELETE RESTRICT,
    number INTEGER NOT NULL CHECK (number >= 1),
    content_json TEXT NOT NULL CHECK (json_valid(content_json)),
    created_at TEXT NOT NULL,
    UNIQUE (project_id, number)
);
ALTER TABLE agent_invocation ADD COLUMN standards_version INTEGER NULL;
ALTER TABLE agent_invocation ADD COLUMN standards_additions_version_id TEXT NULL
    REFERENCES standards_additions_version(id);
