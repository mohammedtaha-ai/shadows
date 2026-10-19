-- The one project AYLUL's conversations live in (AYLUL brain design, section 7).
-- Set only by `Projects::ensure_assistant`; every other project keeps 0.
ALTER TABLE project ADD COLUMN assistant INTEGER NOT NULL DEFAULT 0;
