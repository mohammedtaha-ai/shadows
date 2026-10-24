-- Approval provenance. The command idempotency key remains unchanged.
ALTER TABLE design_vision_proposal ADD COLUMN approved_principal TEXT NULL;
ALTER TABLE design_vision_proposal ADD COLUMN approved_agent TEXT NULL;
ALTER TABLE design_vision_proposal ADD COLUMN approved_request_entry TEXT NULL
    REFERENCES thread_entry(id) ON DELETE RESTRICT;
ALTER TABLE design_map_proposal ADD COLUMN approved_principal TEXT NULL;
ALTER TABLE design_map_proposal ADD COLUMN approved_agent TEXT NULL;
ALTER TABLE design_map_proposal ADD COLUMN approved_request_entry TEXT NULL
    REFERENCES thread_entry(id) ON DELETE RESTRICT;
ALTER TABLE workflow ADD COLUMN approved_principal TEXT NULL;
ALTER TABLE workflow ADD COLUMN approved_agent TEXT NULL;
ALTER TABLE workflow ADD COLUMN approved_request_entry TEXT NULL
    REFERENCES thread_entry(id) ON DELETE RESTRICT;
UPDATE design_vision_proposal SET approved_principal='user:local', approved_agent='human'
    WHERE status='Approved';
UPDATE design_map_proposal SET approved_principal='user:local', approved_agent='human'
    WHERE status='Approved';
-- Temporarily suspend only the workflow UPDATE guard for provenance backfill.
-- Migration execution is transactional; the original guard is restored unchanged.
DROP TRIGGER workflow_frozen_immutable;
UPDATE workflow SET approved_principal='user:local', approved_agent='human'
    WHERE frozen_at IS NOT NULL;
CREATE TRIGGER workflow_frozen_immutable BEFORE UPDATE ON workflow
WHEN OLD.state = 'Frozen'
BEGIN SELECT RAISE(ABORT, 'a frozen plan version never changes'); END;
ALTER TABLE command_record ADD COLUMN audit_principal TEXT NULL;
ALTER TABLE command_record ADD COLUMN audit_agent TEXT NULL;
