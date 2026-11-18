CREATE TABLE external_connection (
    id TEXT PRIMARY KEY,
    token_hash TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    revoked_at TEXT
);

CREATE TABLE external_connection_project (
    connection_id TEXT NOT NULL REFERENCES external_connection(id),
    project_id TEXT NOT NULL REFERENCES project(id),
    grant_id TEXT NOT NULL UNIQUE,
    PRIMARY KEY (connection_id, project_id),
    FOREIGN KEY (grant_id, project_id) REFERENCES mcp_grant(id, project_id)
);

UPDATE mcp_grant SET revoked_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
WHERE kind = 'project' AND revoked_at IS NULL;
