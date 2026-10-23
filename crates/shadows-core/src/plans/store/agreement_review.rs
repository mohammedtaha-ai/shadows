//! Snapshot impact reviews and atomic person agreement.
use super::PlansStore;
use crate::ProjectId;
use crate::db::{CommandContext, StorageError, now};
use crate::design::Design;
use crate::design::{AgreementContent, AgreementId, AgreementState, AgreementVersion};
use crate::plans::{AgreementParticipantImpact, AgreementPartyReview, AgreementReview};
use serde_json::{Value, json};
use sqlx::SqliteConnection;

async fn review(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    id: &AgreementId,
) -> Result<AgreementReview, StorageError> {
    Design::revision_in(conn, project).await?;
    let candidate = Design::agreement_in(conn, project, id, None).await?;
    let base_version = Design::agreed_before_in(conn, project, id, candidate.version).await?;
    let base = match base_version {
        Some(version) => serde_json::to_value(
            Design::agreement_in(conn, project, id, Some(version))
                .await?
                .content,
        )?,
        None => Value::Null,
    };
    let current = serde_json::to_value(&candidate.content)?;
    let mut parties = Vec::new();
    let previous_parties: Vec<crate::AgreementParty> = base
        .get("parties")
        .cloned()
        .map(serde_json::from_value)
        .transpose()?
        .unwrap_or_default();
    let mut declared = candidate.content.parties.clone();
    for party in &previous_parties {
        if !declared.contains(party) {
            declared.push(party.clone());
        }
    }
    for party in &declared {
        let part = Design::part_in(conn, project, &party.part_id).await?;
        parties.push(AgreementPartyReview {
            party: party.clone(),
            revision: part.revision,
            title: part.content.title,
            change: if !candidate.content.parties.contains(party) {
                "Removed"
            } else if !previous_parties.contains(party) {
                "Added"
            } else {
                "Unchanged"
            }
            .into(),
        });
    }
    parties.sort_by_key(|p| (p.party.part_id.to_string(), format!("{:?}", p.party.role)));
    let mut participants = Vec::new();
    let bindings = crate::plans::Plans::agreement_participants_in(conn, project, id).await?;
    for participant in bindings {
        let pin =
            Design::agreement_in(conn, project, id, Some(participant.binding.version)).await?;
        let before = interface(&pin.content, &participant.binding.operations);
        let after = interface(&candidate.content, &participant.binding.operations);
        let changes = serde_json::to_value(json_patch::diff(&before, &after))?;
        let affected = changes.as_array().is_some_and(|v| !v.is_empty())
            || !candidate.content.parties.iter().any(|p| {
                p.part_id == participant.binding.part_id && p.role == participant.binding.role
            });
        let next_action = if !participant.current {
            "Historical binding; preserve the old version"
        } else if participant.plan_state == crate::PlanState::Archived {
            "Unarchive before adoption"
        } else if participant.workflow_state == crate::WorkflowState::Frozen {
            "Continue to a Draft before adoption"
        } else {
            "Review and explicitly edit the Draft binding"
        }
        .into();
        participants.push(AgreementParticipantImpact {
            participant,
            affected,
            changes,
            next_action,
            execution: "Not recorded".into(),
        });
    }
    let basis = json!({"candidate":candidate,"base":base,"parties":parties,
        "participants":participants});
    let review_id = CommandContext::fingerprint("AgreementReview", &basis);
    Ok(AgreementReview {
        agreement_id: id.clone(),
        version: candidate.version,
        revision: candidate.revision,
        review_id,
        base_version,
        compatibility: "Needs review".into(),
        changes: serde_json::to_value(json_patch::diff(&base, &current))?,
        parties,
        participants,
        limits: vec![
            "Unregistered code dependencies are not covered".into(),
            "Structural differences do not certify compatibility or execution".into(),
        ],
    })
}

fn interface(content: &AgreementContent, operations: &[String]) -> Value {
    let mut selected = std::collections::BTreeMap::new();
    if let Some(paths) = content.openapi.get("paths").and_then(Value::as_object) {
        for (path, item) in paths {
            for method in [
                "get", "put", "post", "delete", "options", "head", "patch", "trace",
            ] {
                if let Some(op) = item.get(method)
                    && let Some(id) = op.get("x-shadows-operation-id").and_then(Value::as_str)
                    && operations.iter().any(|n| n == id)
                {
                    selected.insert(
                        id.to_string(),
                        json!({"path":path,"method":method,
                                "operation":op,"parameters":item.get("parameters")}),
                    );
                }
            }
        }
    }
    json!({"operations":selected,"purpose":content.purpose,"behavior":content.behavior,
        "acceptance":content.acceptance,"components":content.openapi.get("components"),
        "security":content.openapi.get("security"),"servers":content.openapi.get("servers")})
}

impl PlansStore {
    pub async fn review_design_agreement(
        &self,
        project: &ProjectId,
        id: &AgreementId,
    ) -> Result<AgreementReview, StorageError> {
        let mut tx = self.storage.reader().begin().await?;
        let result = review(&mut tx, project, id).await?;
        tx.commit().await?;
        Ok(result)
    }
    pub async fn agree_design_agreement(
        &self,
        ctx: &CommandContext,
        project: &ProjectId,
        id: &AgreementId,
        expected_revision: i64,
        review_id: String,
    ) -> Result<AgreementVersion, StorageError> {
        let (ctx, project, id, ts) = (ctx.clone(), project.clone(), id.clone(), now());
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    if let Some(result) = Design::agreement_replay_in(conn, &ctx, &project).await? {
                        return Ok(result);
                    }
                    let basis = review(conn, &project, &id).await?;
                    let candidate = Design::agreement_in(conn, &project, &id, None).await?;
                    if candidate.revision != expected_revision || basis.review_id != review_id {
                        return Err(StorageError::RevisionConflict {
                            current: candidate.revision,
                            summary: "agreement review is stale; review the current participants"
                                .into(),
                        });
                    }
                    if candidate.state != AgreementState::Draft || !candidate.issues.is_empty() {
                        return Err(StorageError::Constraint(
                            "only a valid Draft can be agreed".into(),
                        ));
                    }
                    Design::mark_agreed_in(conn, &id, candidate.version, &ts).await?;
                    let result =
                        Design::agreement_in(conn, &project, &id, Some(candidate.version)).await?;
                    Design::record_agreement_in(conn, &ctx, &project, &result, &ts, true, None)
                        .await
                })
            })
            .await
    }
}
