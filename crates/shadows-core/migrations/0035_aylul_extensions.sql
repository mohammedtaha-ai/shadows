CREATE TABLE aylul_extension (
    id TEXT PRIMARY KEY, kind TEXT NOT NULL CHECK(kind IN ('plugin','skill')),
    name TEXT NOT NULL, description TEXT NOT NULL, digest TEXT NOT NULL,
    created_at TEXT NOT NULL, UNIQUE(kind,name)
);
CREATE TABLE aylul_agent_extension (
    agent_slug TEXT NOT NULL REFERENCES aylul_agent(slug),
    extension_id TEXT NOT NULL REFERENCES aylul_extension(id) ON DELETE CASCADE,
    enabled INTEGER NOT NULL CHECK(enabled IN (0,1)),
    PRIMARY KEY(agent_slug,extension_id)
);
