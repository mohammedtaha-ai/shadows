CREATE TABLE project_extension (
    project_id TEXT NOT NULL REFERENCES project(id),
    extension_id TEXT NOT NULL REFERENCES aylul_extension(id) ON DELETE CASCADE,
    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    PRIMARY KEY (project_id, extension_id)
);
CREATE TABLE project_mcp (
    project_id TEXT NOT NULL REFERENCES project(id),
    mcp_id TEXT NOT NULL REFERENCES aylul_mcp_server(id) ON DELETE CASCADE,
    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    PRIMARY KEY (project_id, mcp_id)
);
ALTER TABLE project ADD COLUMN planner_reads_claude_md INTEGER NOT NULL DEFAULT 0
    CHECK (planner_reads_claude_md IN (0, 1));
