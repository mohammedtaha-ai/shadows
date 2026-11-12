-- The person's language-server settings, for every project. Owned by Codebase.
-- A catalog server the person switched off.
CREATE TABLE code_server_off (
    server_id TEXT PRIMARY KEY,
    at        TEXT NOT NULL
);

-- A server the person added beside the catalog. `args` and `extensions` are JSON lists.
CREATE TABLE code_server_added (
    id         TEXT PRIMARY KEY,
    command    TEXT NOT NULL,
    args       TEXT NOT NULL,
    extensions TEXT NOT NULL,
    at         TEXT NOT NULL
);
