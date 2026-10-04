-- §18.6: exact pins are version content.
CREATE TABLE task_agreement_binding (
    workflow_id TEXT NOT NULL REFERENCES workflow(id) ON DELETE RESTRICT,
    project_id TEXT NOT NULL REFERENCES project(id) ON DELETE RESTRICT,
    task_id TEXT NOT NULL,
    agreement_id TEXT NOT NULL,
    agreement_version INTEGER NOT NULL,
    part_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('provides','uses')),
    operations_json TEXT NOT NULL CHECK(json_valid(operations_json)),
    PRIMARY KEY(workflow_id,task_id,agreement_id,role),
    FOREIGN KEY(task_id,workflow_id) REFERENCES task(id,workflow_id) ON DELETE RESTRICT,
    FOREIGN KEY(project_id,agreement_id,agreement_version)
        REFERENCES agreement_version(project_id,agreement_id,version) ON DELETE RESTRICT,
    FOREIGN KEY(project_id,part_id) REFERENCES design_part(project_id,id) ON DELETE RESTRICT
);
CREATE TRIGGER binding_project_insert BEFORE INSERT ON task_agreement_binding
WHEN NEW.project_id<>(SELECT p.project_id FROM workflow w JOIN plan p ON p.id=w.plan_id
    WHERE w.id=NEW.workflow_id)
BEGIN SELECT RAISE(ABORT,'Binding must belong to the plan project'); END;
CREATE TRIGGER binding_project_update BEFORE UPDATE ON task_agreement_binding
WHEN NEW.project_id<>(SELECT p.project_id FROM workflow w JOIN plan p ON p.id=w.plan_id
    WHERE w.id=NEW.workflow_id)
BEGIN SELECT RAISE(ABORT,'Binding must belong to the plan project'); END;
CREATE TRIGGER binding_frozen_insert BEFORE INSERT ON task_agreement_binding
WHEN (SELECT state FROM workflow WHERE id=NEW.workflow_id)='Frozen'
BEGIN SELECT RAISE(ABORT,'Frozen binding is immutable'); END;
CREATE TRIGGER binding_frozen_update BEFORE UPDATE ON task_agreement_binding
WHEN (SELECT state FROM workflow WHERE id=OLD.workflow_id)='Frozen'
    OR (SELECT state FROM workflow WHERE id=NEW.workflow_id)='Frozen'
BEGIN SELECT RAISE(ABORT,'Frozen binding is immutable'); END;
CREATE TRIGGER binding_frozen_delete BEFORE DELETE ON task_agreement_binding
WHEN (SELECT state FROM workflow WHERE id=OLD.workflow_id)='Frozen'
BEGIN SELECT RAISE(ABORT,'Frozen binding is immutable'); END;
