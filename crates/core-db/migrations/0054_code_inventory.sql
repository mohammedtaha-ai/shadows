-- The file inventory is derived from the folder, as the index is: it can be
-- deleted and rebuilt, and carries no command or journal row.

CREATE TABLE code_inventory (
    project_id      TEXT NOT NULL REFERENCES project(id) ON DELETE RESTRICT,
    path_key        TEXT NOT NULL,          -- `path`, lowercased on Windows
    path            TEXT NOT NULL,          -- relative to the project folder, '/'
    kind            TEXT NOT NULL CHECK (kind IN ('directory','file','excluded')),
    excluded_reason TEXT NULL,
    PRIMARY KEY (project_id, path_key),
    CHECK ((kind = 'excluded') = (excluded_reason IS NOT NULL))
);

CREATE TABLE code_inventory_scan (
    project_id  TEXT PRIMARY KEY REFERENCES project(id) ON DELETE RESTRICT,
    generation  INTEGER NOT NULL CHECK (generation >= 1),
    incomplete  INTEGER NOT NULL CHECK (incomplete IN (0, 1)),
    observed_at TEXT NOT NULL
);
