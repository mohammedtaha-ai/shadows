-- Efforts belong to a model (§12.4): per harness and model, the effort of the
-- last turn started on that model. Not journal data: one row per pair, latest
-- wins. A model a turn ran without an effort has no row.
CREATE TABLE harness_model_effort (
    harness_kind TEXT NOT NULL,
    model        TEXT NOT NULL,
    effort       TEXT NOT NULL,
    updated_at   TEXT NOT NULL,
    PRIMARY KEY (harness_kind, model)
);

-- The effort each harness remembered belonged to the model it remembered:
-- it moves to that model's row, and `harness_preference` keeps the model only.
INSERT INTO harness_model_effort (harness_kind, model, effort, updated_at)
SELECT harness_kind, model, effort, updated_at
  FROM harness_preference
 WHERE effort IS NOT NULL;

ALTER TABLE harness_preference DROP COLUMN effort;
