-- A project owns the directory its turns run in (spec §4.2, §6.3).
--
-- Rows written before this migration have no directory, and none can be
-- backfilled: the only candidate is the directory the daemon happened to be
-- started in, which was never recorded and is not the user's choice. They keep
-- NULL, and a turn on such a project fails at Prepare (spec §8.3) instead of
-- silently running somewhere nobody picked.
--
-- Every row written from now on must carry one. `ALTER TABLE ... ADD COLUMN`
-- cannot add a NOT NULL column without a default, and a default would be the
-- invented value refused above, so the rule for new rows is enforced by
-- triggers instead: NULL can be neither inserted nor written over a directory.
ALTER TABLE project ADD COLUMN directory TEXT NULL;

CREATE TRIGGER project_directory_required
BEFORE INSERT ON project
WHEN NEW.directory IS NULL
BEGIN
    SELECT RAISE(ABORT, 'project.directory is required');
END;

CREATE TRIGGER project_directory_not_cleared
BEFORE UPDATE OF directory ON project
WHEN NEW.directory IS NULL
BEGIN
    SELECT RAISE(ABORT, 'project.directory cannot be cleared');
END;
