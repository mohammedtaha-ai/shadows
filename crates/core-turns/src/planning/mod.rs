//! Planning context compiled through its design, plan and code record owners.
mod block;
mod budget;

use crate::{Turns, TurnsError};
use block::{Record, RecordFile, RecordGroup, RecordSymbol, SourceScope};
use code_record::{CodeRecord, CodeRecordError, DigestFile, RecordDigest};
use core_design::{Design, DesignAnchor};
use core_harness::CarriedContext;
use core_plans::{PlanId, Plans};
use core_projects::ProjectId;
use core_threads::ThreadId;

impl Turns {
    pub async fn planning_context(
        &self,
        thread: &ThreadId,
        project: &ProjectId,
    ) -> Result<Option<CarriedContext>, TurnsError> {
        let context = core_threads::Threads::turn_context_in(
            &mut *self.storage.reader().begin().await?,
            thread,
        )
        .await?;
        // An executor reads its one task through `task_context`, never the plan around it.
        if context.harness == "aylul" || context.kind == core_threads::ThreadKind::Executor {
            return Ok(None);
        }
        tracing::debug!(%thread, "planner.context_compiling");
        Ok(Some(
            Planning::compile(
                &self.design,
                &self.plans,
                &self.code_record,
                thread,
                project,
                [
                    budget::INDEX_TOKENS,
                    budget::DETAIL_TOKENS,
                    budget::RECORD_TOKENS,
                ],
            )
            .await?,
        ))
    }
}

pub struct Planning;

impl Planning {
    /// `budgets`: the index's, the detail's and the code record's, in estimated tokens.
    pub async fn compile(
        design: &Design,
        plans: &Plans,
        code_record: &CodeRecord,
        thread: &ThreadId,
        project: &ProjectId,
        budgets: [i64; 3],
    ) -> Result<CarriedContext, TurnsError> {
        let [index_budget, detail_budget, record_budget] = budgets;
        let mut block = block::Block::default();
        let vision = design.vision(project).await?;
        block.source("vision", project.as_str(), vision.revision);
        for item in vision.content.purpose {
            block
                .index
                .push(format!("Purpose {}: {}", item.id, item.text));
        }
        let parts = design.parts(project, None, None).await?;
        for part in &parts.items {
            block
                .index
                .push(format!("Part {}: {}", part.id, part.content.title));
            block.source("part", part.id.as_str(), part.revision);
            let children = design.parts(project, Some(&part.id), None).await?;
            for child in children.items {
                block.part_children.push(format!(
                    "  Part {} (parent {}): {}",
                    child.id, part.id, child.content.title
                ));
                block.scoped_source(
                    SourceScope::PartChildren,
                    "part",
                    child.id.as_str(),
                    child.revision,
                );
            }
            if children.next.is_some() {
                block.part_children.push("[more parts available]".into());
            }
        }
        if parts.next.is_some() {
            block.index.push("[more parts available]".into());
        }
        let outcomes = design.outcomes(project, None, None).await?;
        for outcome in &outcomes.items {
            block
                .index
                .push(format!("Outcome {}: {}", outcome.id, outcome.content.title));
            block.source("outcome", outcome.id.as_str(), outcome.revision);
            let children = design.outcomes(project, Some(&outcome.id), None).await?;
            for child in children.items {
                block.outcome_children.push(format!(
                    "  Outcome {} (parent {}): {}",
                    child.id, outcome.id, child.content.title
                ));
                block.scoped_source(
                    SourceScope::OutcomeChildren,
                    "outcome",
                    child.id.as_str(),
                    child.revision,
                );
            }
            if children.next.is_some() {
                block
                    .outcome_children
                    .push("[more outcomes available]".into());
            }
        }
        if outcomes.next.is_some() {
            block.index.push("[more outcomes available]".into());
        }
        for mut agreement in design.agreements(project).await? {
            while agreement.state != core_design::AgreementState::Agreed && agreement.version > 1 {
                agreement = design
                    .agreement(
                        project,
                        &agreement.agreement_id,
                        Some(agreement.version - 1),
                    )
                    .await?;
            }
            if agreement.state == core_design::AgreementState::Agreed {
                block.index_agreements.push(format!(
                    "Agreement {} v{}: {}",
                    agreement.agreement_id, agreement.version, agreement.content.capability
                ));
                block.scoped_source(
                    SourceScope::IndexAgreements,
                    "agreement",
                    agreement.agreement_id.as_str(),
                    agreement.version,
                );
            }
        }
        if let Some(anchor) = design.thread_anchor(thread).await?
            && !Self::detail(&mut block, design, plans, project, &anchor).await?
        {
            let (kind, id) = match &anchor {
                DesignAnchor::Part(id) => ("part", id.as_str()),
                DesignAnchor::Outcome(id) => ("outcome", id.as_str()),
            };
            block.index.push(format!("[anchor removed: {kind} {id}]"));
        }
        let digest = code_record
            .digest(project)
            .await
            .map_err(Self::record_error)?;
        // The count of done marks versions the record, so a new mark re-sends it.
        block.source("code_record", project.as_str(), i64::from(digest.marks));
        block.record = Some(Self::record(&digest));
        Ok(block.finish(index_budget, detail_budget, record_budget))
    }

    fn record_error(error: CodeRecordError) -> TurnsError {
        match error {
            CodeRecordError::Storage(error) => error.into(),
            CodeRecordError::Design(error) => error.into(),
            CodeRecordError::InvalidTopic(why) => TurnsError::InvalidCommand(why),
        }
    }

    /// The digest as the block's lines: a group per part, then the files no part owns.
    fn record(digest: &RecordDigest) -> Record {
        let mut record = Record::default();
        for part in &digest.parts {
            record.groups.push(RecordGroup::new(
                format!("Part {} ({}):", part.title, part.part_id),
                part.title.clone(),
                part.files.iter().map(Self::record_file).collect(),
            ));
        }
        if !digest.unowned.is_empty() {
            record.groups.push(RecordGroup::new(
                "Files no part owns:".into(),
                "no part".into(),
                digest.unowned.iter().map(Self::record_file).collect(),
            ));
        }
        record.gaps = digest
            .gaps
            .iter()
            .map(|gap| {
                format!(
                    "{} {} {} ({}): {}",
                    gap.element_id,
                    Self::word(&gap.kind),
                    gap.name,
                    Self::word(&gap.state),
                    gap.why
                )
            })
            .collect();
        record
    }

    fn record_file(file: &DigestFile) -> RecordFile {
        let layer = file
            .layer
            .as_ref()
            .map(|layer| format!(" [{layer}]"))
            .unwrap_or_default();
        RecordFile {
            line: format!("  {}{layer}: {}", file.path, file.purpose),
            symbols: file
                .symbols
                .iter()
                .map(|symbol| {
                    let inputs: Vec<String> = symbol
                        .inputs
                        .iter()
                        .map(|input| format!("{}: {}", input.name, input.type_name))
                        .collect();
                    let signature = match (&symbol.returns, inputs.is_empty()) {
                        (None, true) => String::new(),
                        (None, false) => format!(": {}", inputs.join(", ")),
                        (Some(returns), true) => format!(": () -> {}", returns.type_name),
                        (Some(returns), false) => {
                            format!(": {} -> {}", inputs.join(", "), returns.type_name)
                        }
                    };
                    RecordSymbol {
                        named: format!("{} ({})", symbol.name, Self::word(&symbol.kind)),
                        signature,
                    }
                })
                .collect(),
        }
    }

    /// A snake_case enum value as it reads: `function`, `gap`.
    fn word(value: &impl serde::Serialize) -> String {
        match serde_json::to_value(value) {
            Ok(serde_json::Value::String(word)) => word,
            Ok(other) => other.to_string(),
            Err(_) => String::new(),
        }
    }

    async fn detail(
        block: &mut block::Block,
        design: &Design,
        plans: &Plans,
        project: &ProjectId,
        anchor: &DesignAnchor,
    ) -> Result<bool, TurnsError> {
        let linked: Vec<PlanId> = match anchor {
            DesignAnchor::Part(id) => {
                let view = match plans.design_part(project, id).await {
                    Ok(view) => view,
                    Err(core_plans::PlansError::Storage(core_db::StorageError::NotFound(_))) => {
                        return Ok(false);
                    }
                    Err(error) => return Err(error.into()),
                };
                block
                    .detail
                    .push(serde_json::to_string(&view.part.content)?);
                block.source("part", id.as_str(), view.part.revision);
                for parent in view.ancestors {
                    block
                        .detail
                        .push(format!("Ancestor {}: {}", parent.id, parent.content.title));
                    block.source("part", parent.id.as_str(), parent.revision);
                }
                let children = design.parts(project, Some(id), None).await?;
                for child in children.items {
                    block
                        .detail
                        .push(format!("Child {}: {}", child.id, child.content.title));
                    block.children.push(format!(
                        "Child {} responsibility: {}",
                        child.id, child.content.responsibility
                    ));
                    block.source("part", child.id.as_str(), child.revision);
                }
                if children.next.is_some() {
                    block.detail.push("[more parts available]".into());
                }
                view.plans
            }
            DesignAnchor::Outcome(id) => {
                let view = match plans.design_outcome(project, id).await {
                    Ok(view) => view,
                    Err(core_plans::PlansError::Storage(core_db::StorageError::NotFound(_))) => {
                        return Ok(false);
                    }
                    Err(error) => return Err(error.into()),
                };
                block
                    .detail
                    .push(serde_json::to_string(&view.outcome.content)?);
                block.source("outcome", id.as_str(), view.outcome.revision);
                for parent in view.ancestors {
                    block
                        .detail
                        .push(format!("Ancestor {}: {}", parent.id, parent.content.title));
                    block.source("outcome", parent.id.as_str(), parent.revision);
                }
                let children = design.outcomes(project, Some(id), None).await?;
                for child in children.items {
                    block
                        .detail
                        .push(format!("Child {}: {}", child.id, child.content.title));
                    block.children.push(format!(
                        "Child {} intended result: {}",
                        child.id, child.content.intended_result
                    ));
                    block.source("outcome", child.id.as_str(), child.revision);
                }
                if children.next.is_some() {
                    block.detail.push("[more outcomes available]".into());
                }
                for part in view.parts {
                    let view = plans.design_part(project, &part).await?;
                    block
                        .detail
                        .push(format!("Part {}: {}", part, view.part.content.title));
                    block.source("part", part.as_str(), view.part.revision);
                }
                view.plans
            }
        };
        for listing in plans.list(project, false).await? {
            if !linked.contains(&listing.plan_id) {
                continue;
            }
            let plan = plans.get(&listing.id).await?;
            block.source("plan", plan.id.as_str(), plan.version);
            block.detail.push(format!(
                "Plan {} v{}: {}\nGoal: {}",
                plan.plan_id, plan.version, plan.title, plan.goal
            ));
            for task in &plan.tasks {
                block
                    .detail
                    .push(format!("T{}: {}", task.content.number, task.content.title));
            }
            if let Some(reason) = &plan.change_reason {
                block.detail.push(format!("Rationale: {reason}"));
            }
            let mut previous = plan.previous;
            for _ in 0..2 {
                let Some(id) = previous else { break };
                let earlier = plans.get(&id).await?;
                if let Some(reason) = earlier.change_reason {
                    block.older.push(format!(
                        "Earlier rationale for plan {} v{}: {}",
                        earlier.plan_id, earlier.version, reason
                    ));
                    block.scoped_source(
                        SourceScope::OlderRationales,
                        "plan",
                        earlier.id.as_str(),
                        earlier.version,
                    );
                }
                previous = earlier.previous;
            }
            for binding in plan.bindings {
                let agreement = design
                    .agreement(project, &binding.agreement_id, Some(binding.version))
                    .await?;
                let header = format!(
                    "Bound agreement {} v{}: {}",
                    agreement.agreement_id, agreement.version, agreement.content.capability
                );
                if !block.detail.contains(&header) {
                    block.detail.push(header);
                    block
                        .agreement_bodies
                        .push(serde_json::to_string(&agreement.content)?);
                    block.source(
                        "agreement",
                        agreement.agreement_id.as_str(),
                        agreement.version,
                    );
                }
            }
        }
        Ok(true)
    }
}
