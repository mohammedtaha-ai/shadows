-- Each write appends a revision. A tombstone is the element's final revision.
CREATE TABLE design_elements (
    project_id TEXT NOT NULL,
    element_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK (revision >= 1),
    kind TEXT NOT NULL CHECK (kind IN ('table', 'screen')),
    name TEXT NOT NULL,
    part_id TEXT NOT NULL,
    outcomes_json TEXT NOT NULL CHECK (json_valid(outcomes_json)),
    content_json TEXT NOT NULL CHECK (json_valid(content_json)),
    deleted INTEGER NOT NULL DEFAULT 0 CHECK (deleted IN (0, 1)),
    written_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (project_id, element_id, revision)
);
