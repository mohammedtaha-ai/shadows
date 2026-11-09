-- Codex's one mode for existing ordinary projects (spec §12.14): filesystem
-- read-only, the least authority Codex runs with. No write mode is added, and
-- no other harness's modes change. AYLUL's project runs only AYLUL.
INSERT OR IGNORE INTO project_mode (project_id, harness_kind, mode_id)
    SELECT id, 'codex', 'read-only' FROM project WHERE assistant = 0;
