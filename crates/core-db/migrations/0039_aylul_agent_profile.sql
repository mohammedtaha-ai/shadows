ALTER TABLE aylul_agent ADD COLUMN description TEXT NOT NULL DEFAULT '';
ALTER TABLE aylul_agent ADD COLUMN instructions TEXT;
ALTER TABLE aylul_agent ADD COLUMN outfit TEXT NOT NULL DEFAULT 'default';

ALTER TABLE aylul_extension
    ADD COLUMN bundled INTEGER NOT NULL DEFAULT 0 CHECK (bundled IN (0, 1));

CREATE TABLE aylul_mcp_server (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL,
    config_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE aylul_agent_mcp (
    agent_slug TEXT NOT NULL REFERENCES aylul_agent(slug),
    mcp_id TEXT NOT NULL REFERENCES aylul_mcp_server(id) ON DELETE CASCADE,
    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    PRIMARY KEY (agent_slug, mcp_id)
);

CREATE TABLE aylul_extension_declined (
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    declined_at TEXT NOT NULL,
    PRIMARY KEY (kind, name)
);
