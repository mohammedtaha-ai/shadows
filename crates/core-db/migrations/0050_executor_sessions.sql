-- An executor session runs in a thread of its own kind, and its grant names the
-- one task it may read. SQLite cannot alter a CHECK, so `mcp_grant` is rebuilt;
-- `defer_foreign_keys` lets `DROP TABLE` leave `draft_intent` and `plan_version`
-- pointing at nothing until the new table holds the same ids, before the commit.
PRAGMA defer_foreign_keys = ON;

ALTER TABLE planning_thread ADD COLUMN kind TEXT NOT NULL DEFAULT 'planning'
    CHECK (kind IN ('planning','executor'));

CREATE TEMP TABLE mcp_grant_before AS SELECT * FROM mcp_grant;
DROP TABLE mcp_grant;

CREATE TABLE mcp_grant (
    id             TEXT PRIMARY KEY,
    kind           TEXT NOT NULL CHECK (kind IN ('thread','project','executor')),
    thread_id      TEXT NULL,
    project_id     TEXT NOT NULL REFERENCES project(id),
    token_hash     TEXT NOT NULL UNIQUE,
    created_at     TEXT NOT NULL,
    revoked_at     TEXT NULL,
    scope_workflow TEXT NULL,
    scope_version  INTEGER NULL,
    scope_task     INTEGER NULL,
    UNIQUE (id, project_id),
    CHECK ((kind = 'project') = (thread_id IS NULL)),
    CHECK ((kind = 'executor') = (scope_workflow IS NOT NULL)),
    CHECK ((scope_workflow IS NULL) = (scope_version IS NULL)),
    CHECK ((scope_workflow IS NULL) = (scope_task IS NULL)),
    FOREIGN KEY (thread_id, project_id) REFERENCES planning_thread(id, project_id)
);

INSERT INTO mcp_grant (id, kind, thread_id, project_id, token_hash, created_at, revoked_at)
SELECT id, kind, thread_id, project_id, token_hash, created_at, revoked_at FROM mcp_grant_before;
DROP TABLE mcp_grant_before;
