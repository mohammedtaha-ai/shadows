-- §23.7: intent precedes disk writes; completion follows every file.
CREATE TABLE design_structure_write (
    project_id TEXT PRIMARY KEY REFERENCES design_workspace(project_id) ON DELETE RESTRICT,
    map_number INTEGER NOT NULL,
    standards_version INTEGER NOT NULL,
    standards_additions_version_id TEXT NULL REFERENCES standards_additions_version(id),
    files_json TEXT NOT NULL CHECK (json_valid(files_json)),
    folders_json TEXT NOT NULL CHECK (json_valid(folders_json)),
    started_at TEXT NOT NULL,
    completed_at TEXT NULL
);
