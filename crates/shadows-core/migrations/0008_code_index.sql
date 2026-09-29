-- Milestone 3 (§15.4). The index is derived from the files: it can be deleted
-- and rebuilt, and carries no command or journal row.

CREATE TABLE code_file (
    project_id     TEXT NOT NULL REFERENCES project(id) ON DELETE RESTRICT,
    path_key       TEXT NOT NULL,          -- `path`, lowercased on Windows
    path           TEXT NOT NULL,          -- relative to the project folder, '/'
    size           INTEGER NOT NULL,
    modified_ms    INTEGER NOT NULL,
    language       TEXT NOT NULL,
    skipped_reason TEXT NULL CHECK (skipped_reason IN ('too_large','binary','not_utf8')),
    PRIMARY KEY (project_id, path_key)
);

CREATE TABLE code_tag (
    project_id TEXT NOT NULL,
    path_key   TEXT NOT NULL,
    path       TEXT NOT NULL,
    name       TEXT NOT NULL,
    kind       TEXT NOT NULL,
    role       TEXT NOT NULL CHECK (role IN ('definition','reference')),
    line       INTEGER NOT NULL CHECK (line >= 1),
    signature  TEXT NULL,
    FOREIGN KEY (project_id, path_key) REFERENCES code_file(project_id, path_key) ON DELETE CASCADE
);
CREATE INDEX code_tag_name ON code_tag(project_id, name);
CREATE INDEX code_tag_path ON code_tag(project_id, path_key, line);

CREATE TABLE code_setting (
    id           INTEGER PRIMARY KEY CHECK (id = 1),
    active_limit INTEGER NOT NULL CHECK (active_limit BETWEEN 1 AND 20)
);
INSERT INTO code_setting (id, active_limit) VALUES (1, 5);

CREATE TABLE project_link (
    project_id        TEXT NOT NULL REFERENCES project(id) ON DELETE RESTRICT,
    linked_project_id TEXT NOT NULL REFERENCES project(id) ON DELETE RESTRICT,
    created_at        TEXT NOT NULL,
    PRIMARY KEY (project_id, linked_project_id),
    CHECK (project_id <> linked_project_id)
);
