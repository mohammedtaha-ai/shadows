ALTER TABLE command_record
ADD COLUMN request_payload JSONB NOT NULL DEFAULT '{}'::jsonb;
