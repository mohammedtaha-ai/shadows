ALTER TABLE external_connection ADD COLUMN name TEXT;
ALTER TABLE external_connection ADD COLUMN project_revision INTEGER NOT NULL DEFAULT 0;
