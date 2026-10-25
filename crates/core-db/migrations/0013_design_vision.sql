CREATE TABLE design_workspace (
    project_id TEXT PRIMARY KEY REFERENCES project(id) ON DELETE RESTRICT,
    revision INTEGER NOT NULL CHECK (revision >= 0),
    vision_revision INTEGER NOT NULL CHECK (vision_revision >= 0),
    vision_content TEXT NOT NULL CHECK (json_valid(vision_content))
);

CREATE TABLE design_command_result (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES project(id) ON DELETE RESTRICT,
    result_json TEXT NOT NULL CHECK (json_valid(result_json))
);

CREATE TRIGGER design_result_no_update BEFORE UPDATE ON design_command_result
BEGIN SELECT RAISE(ABORT, 'design command results are immutable'); END;
CREATE TRIGGER design_result_no_delete BEFORE DELETE ON design_command_result
BEGIN SELECT RAISE(ABORT, 'design command results are immutable'); END;
