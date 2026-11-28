//! Project decisions: a project agent proposes a revision and only the person
//! accepts it, which supersedes the id's earlier accepted revision.
use super::Design;
use crate::{DecisionInput, DecisionRecord, DesignError, Grant, ProjectId};
use core_db::CommandContext;

impl Design {
    /// A proposed revision with a project event and a card for a Planner's turn.
    pub async fn propose_decision(
        &self,
        grant: &Grant,
        command_id: String,
        input: DecisionInput,
    ) -> Result<DecisionRecord, DesignError> {
        let (writer, turn) = self
            .proposal_writer(
                grant,
                "decision_propose needs a Planner's conversation",
                "no turn is running for this conversation",
            )
            .await?;
        let command_id = if turn.is_none() {
            Self::external_command_id(Some(command_id))?
        } else {
            command_id
        };
        let (principal_kind, principal_id) = writer.principal();
        let ctx = CommandContext {
            agent: None,
            principal_kind: principal_kind.into(),
            principal_id,
            command_id,
            command_kind: "DecisionPropose".into(),
            command_schema_ver: 1,
            request_fingerprint: CommandContext::fingerprint(
                "DecisionPropose",
                &serde_json::to_value(&input)?,
            ),
        };
        self.store
            .propose_decision_in(&ctx, &writer, &grant.project_id, turn.as_ref(), input)
            .await
    }

    /// Every revision of every decision of `project`, by id then version.
    pub async fn decisions(&self, project: &ProjectId) -> Result<Vec<DecisionRecord>, DesignError> {
        self.store.decisions(project).await
    }

    /// The person accepts the latest, proposed revision `version` of `id`.
    pub async fn accept_decision(
        &self,
        command_id: String,
        project: &ProjectId,
        id: &str,
        version: i64,
    ) -> Result<DecisionRecord, DesignError> {
        let ctx = CommandContext::user(
            command_id,
            "DecisionAccept",
            serde_json::json!({"project":project,"decision":id,"version":version}),
        );
        self.store
            .accept_decision_in(&ctx, project, id, version)
            .await
    }

    /// One revision as `conn` sees it, for a caller's own write.
    pub async fn decision_in(
        conn: &mut sqlx::SqliteConnection,
        project: &ProjectId,
        id: &str,
        version: i64,
    ) -> Result<Option<DecisionRecord>, DesignError> {
        crate::store::DesignStore::decision_in(conn, project, id, version).await
    }
}

/// The text limits a proposed decision must hold.
pub(crate) struct DecisionRules;

impl DecisionRules {
    /// The input with its texts trimmed, or why it is refused.
    pub(crate) fn normalise(mut input: DecisionInput) -> Result<DecisionInput, DesignError> {
        for (name, text, min, max) in [
            ("title", &mut input.title, 1, 200),
            ("decision", &mut input.decision, 1, 4_000),
            ("rationale", &mut input.rationale, 1, 4_000),
            ("source", &mut input.source, 0, 1_000),
        ] {
            *text = text.trim().to_string();
            let length = text.chars().count();
            if length < min || length > max {
                return Err(DesignError::InvalidDecision(format!(
                    "a decision's {name} needs {min} to {max} characters, not {length}"
                )));
            }
        }
        if let Some(id) = &mut input.id {
            *id = id.trim().to_string();
        }
        match (&input.id, input.expected_version) {
            (None, Some(_)) => Err(DesignError::InvalidDecision(
                "a new decision has no expected_version".into(),
            )),
            (Some(id), None) => Err(DesignError::DecisionStale(format!(
                "{id} needs the expected_version of its latest revision"
            ))),
            _ => Ok(input),
        }
    }
}
