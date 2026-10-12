-- Approved maps, immutable snapshots, person-approved waivers and per-part drift.
ALTER TABLE design_workspace ADD COLUMN map_revision INTEGER NOT NULL DEFAULT 0
    CHECK (map_revision >= 0);
ALTER TABLE design_part ADD COLUMN served_revisions TEXT NOT NULL DEFAULT '{}'
    CHECK (json_valid(served_revisions));
CREATE TABLE design_map_version (
    project_id TEXT NOT NULL REFERENCES design_workspace(project_id) ON DELETE RESTRICT,
    number INTEGER NOT NULL CHECK (number >= 1),
    parts_json TEXT NOT NULL CHECK (json_valid(parts_json)),
    vision_revision INTEGER NOT NULL,
    proposal_id TEXT NULL,
    standards_version INTEGER NOT NULL,
    standards_additions_version_id TEXT NULL REFERENCES standards_additions_version(id),
    created_at TEXT NOT NULL,
    PRIMARY KEY (project_id,number)
);
CREATE TABLE design_part_waiver (
    project_id TEXT NOT NULL REFERENCES design_workspace(project_id) ON DELETE RESTRICT,
    part_name TEXT NOT NULL,
    reason TEXT NOT NULL CHECK (length(trim(reason)) > 0),
    approved_by TEXT NOT NULL,
    proposal_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (project_id,part_name)
);
