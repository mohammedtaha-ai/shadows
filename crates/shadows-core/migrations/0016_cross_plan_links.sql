-- §16.7: source versions own their links; targets name a plan/task number.
CREATE TABLE task_plan_parent (
    workflow_id       TEXT NOT NULL REFERENCES workflow(id) ON DELETE RESTRICT,
    task_id           TEXT NOT NULL,
    parent_plan_id    TEXT NOT NULL REFERENCES plan(id) ON DELETE RESTRICT,
    parent_task       INTEGER NOT NULL CHECK (parent_task >= 1),
    kind              TEXT NOT NULL CHECK (kind IN ('needs','completes_after')),
    label             TEXT NOT NULL,
    waiting_items     TEXT NULL,
    PRIMARY KEY (workflow_id, task_id, parent_plan_id, parent_task, kind),
    FOREIGN KEY (task_id, workflow_id) REFERENCES task(id, workflow_id) ON DELETE RESTRICT,
    CHECK ((kind = 'completes_after') = (waiting_items IS NOT NULL))
);
CREATE INDEX task_plan_parent_by_target ON task_plan_parent(parent_plan_id, parent_task);

CREATE TRIGGER task_plan_parent_frozen_insert BEFORE INSERT ON task_plan_parent
WHEN (SELECT state FROM workflow WHERE id = NEW.workflow_id) = 'Frozen'
BEGIN SELECT RAISE(ABORT, 'a frozen plan version never changes'); END;

CREATE TRIGGER task_plan_parent_frozen_update BEFORE UPDATE ON task_plan_parent
WHEN (SELECT state FROM workflow WHERE id = OLD.workflow_id) = 'Frozen'
  OR (SELECT state FROM workflow WHERE id = NEW.workflow_id) = 'Frozen'
BEGIN SELECT RAISE(ABORT, 'a frozen plan version never changes'); END;

CREATE TRIGGER task_plan_parent_frozen_delete BEFORE DELETE ON task_plan_parent
WHEN (SELECT state FROM workflow WHERE id = OLD.workflow_id) = 'Frozen'
BEGIN SELECT RAISE(ABORT, 'a frozen plan version never changes'); END;
