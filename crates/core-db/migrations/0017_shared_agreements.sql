-- §18.4: versioned agreements with one editable Draft per identity.
CREATE TABLE agreement (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES project(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL,
    UNIQUE(project_id,id)
);
CREATE TABLE agreement_version (
    agreement_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    version INTEGER NOT NULL CHECK(version>0),
    revision INTEGER NOT NULL CHECK(revision>=0),
    state TEXT NOT NULL CHECK(state IN ('Draft','Agreed')),
    reason TEXT,
    writer_json TEXT NOT NULL CHECK(json_valid(writer_json)),
    created_at TEXT NOT NULL,
    agreed_at TEXT,
    content_json TEXT NOT NULL CHECK(json_valid(content_json)),
    PRIMARY KEY(agreement_id,version),
    FOREIGN KEY(project_id,agreement_id) REFERENCES agreement(project_id,id) ON DELETE RESTRICT,
    CHECK(version=1 OR length(trim(reason))>0),
    CHECK((state='Draft' AND agreed_at IS NULL) OR (state='Agreed' AND agreed_at IS NOT NULL))
);
CREATE UNIQUE INDEX agreement_one_draft ON agreement_version(agreement_id) WHERE state='Draft';
CREATE UNIQUE INDEX agreement_project_version ON agreement_version(project_id,agreement_id,version);
CREATE TRIGGER agreement_agreed_update BEFORE UPDATE ON agreement_version
WHEN OLD.state='Agreed'
BEGIN SELECT RAISE(ABORT,'Agreed agreement version is immutable'); END;
CREATE TRIGGER agreement_agreed_delete BEFORE DELETE ON agreement_version
WHEN OLD.state='Agreed'
BEGIN SELECT RAISE(ABORT,'Agreed agreement version is immutable'); END;
CREATE TRIGGER agreement_no_agreed_insert BEFORE INSERT ON agreement_version
WHEN NEW.state='Agreed'
BEGIN SELECT RAISE(ABORT,'Start an agreement as Draft before agreement'); END;
CREATE TABLE agreement_operation_identity (
    agreement_id TEXT NOT NULL REFERENCES agreement(id) ON DELETE RESTRICT,
    operation_id TEXT NOT NULL,
    PRIMARY KEY(agreement_id,operation_id)
);
