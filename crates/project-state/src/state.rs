//! One job: composing the project's state from its owners' reads.

use std::collections::HashMap;

use code_record::{BuildState, ElementState, PlannedPin};
use core_design::{DesignElement, PartId, ProjectLayout, StageView};
use core_plans::LiveVersion;
use core_projects::ProjectId;
use core_runs::TaskProgress;

use crate::model::{
    ElementGap, ElementView, PartDetail, PartState, PlanProgress, PlanTasks, ProjectStateView,
    StateCounts, TaskCounts, TaskLine,
};
use crate::{ProjectState, ProjectStateError};

/// At most this many parts in the state.
const PARTS_MAX: usize = 200;
/// At most this many plans under one part, and unassigned.
const PLANS_PER_PART: usize = 20;
/// At most this many gaps.
const GAPS_MAX: usize = 100;
/// At most this many plans in a part's detail.
const DETAIL_PLANS_MAX: usize = 50;

/// What every read of the state is composed from, read in one fixed order.
struct Facts {
    stage: StageView,
    layout: ProjectLayout,
    elements: Vec<DesignElement>,
    live: Vec<LiveVersion>,
    /// The progress of `live[i]`'s live version; empty for a plan with only a Draft.
    progress: Vec<Vec<TaskProgress>>,
    /// The state of `elements[i]`.
    states: Vec<ElementState>,
}

impl ProjectState {
    /// The project's stage, its parts in map order with their plans and elements counted by
    /// state, the plans tied to no part, the gaps, and with `part` that part in full.
    pub async fn state(
        &self,
        project: &ProjectId,
        part: Option<&PartId>,
    ) -> Result<ProjectStateView, ProjectStateError> {
        let facts = self.facts(project).await?;
        if let Some(part) = part
            && !facts.layout.parts.iter().any(|p| &p.id == part)
        {
            return Err(ProjectStateError::UnknownPart(format!(
                "part {part} is not a part of this project"
            )));
        }
        let plans: Vec<PlanProgress> = facts
            .live
            .iter()
            .zip(&facts.progress)
            .map(|(live, progress)| Self::plan_state(live, progress))
            .collect();
        let mut parts = Vec::new();
        for layout_part in facts.layout.parts.iter().take(PARTS_MAX) {
            let (listed, plans_more) = Self::bounded(
                facts
                    .live
                    .iter()
                    .zip(&plans)
                    .filter(|(live, _)| live.parts.contains(&layout_part.id))
                    .map(|(_, plan)| plan.clone()),
                PLANS_PER_PART,
            );
            let mut counts = StateCounts::default();
            for (element, state) in facts.elements.iter().zip(&facts.states) {
                if element.part_id == layout_part.id {
                    match state.state {
                        BuildState::Planned => counts.planned += 1,
                        BuildState::Built => counts.built += 1,
                        BuildState::Gap => counts.gap += 1,
                        BuildState::Unchecked => counts.unchecked += 1,
                        BuildState::Removed => {}
                    }
                }
            }
            parts.push(PartState {
                part_id: layout_part.id.clone(),
                parent: layout_part.parent.clone(),
                title: layout_part.title.clone(),
                kind: layout_part.kind.clone(),
                plans: listed,
                plans_more,
                elements: counts,
            });
        }
        let parts_more = Self::more(facts.layout.parts.len(), PARTS_MAX);
        let (unassigned, unassigned_more) = Self::bounded(
            facts
                .live
                .iter()
                .zip(&plans)
                .filter(|(live, _)| live.parts.is_empty())
                .map(|(_, plan)| plan.clone()),
            PLANS_PER_PART,
        );
        let (gaps, gaps_more) = Self::bounded(Self::gaps(&facts).into_iter(), GAPS_MAX);
        let part = part.map(|part| Self::detail(&facts, part));
        Ok(ProjectStateView {
            project_id: project.clone(),
            stage: facts.stage.stage,
            missing: facts.stage.missing,
            map_revision: facts.stage.map_revision,
            parts,
            parts_more,
            unassigned,
            unassigned_more,
            gaps,
            gaps_more,
            part,
        })
    }

    /// Every element of the project with its state, by element number (at most 500).
    pub async fn elements(
        &self,
        project: &ProjectId,
    ) -> Result<Vec<ElementView>, ProjectStateError> {
        let facts = self.facts(project).await?;
        Ok(facts
            .elements
            .into_iter()
            .zip(facts.states)
            .map(|(element, state)| ElementView { element, state })
            .collect())
    }

    /// Design's stage, layout and elements; Plans' live versions; Runs' progress of each; then
    /// CodeRecord's element states over the pins of the live tasks not done.
    async fn facts(&self, project: &ProjectId) -> Result<Facts, ProjectStateError> {
        let stage = self.design.stage(project).await?;
        let layout = self.design.layout(project).await?;
        let elements = self.design.elements(project).await?;
        let live = self.plans.live_versions(project).await?;
        let mut progress = Vec::with_capacity(live.len());
        for plan in &live {
            progress.push(match &plan.live {
                Some(frozen) => {
                    self.runs
                        .progress(&frozen.workflow_id, frozen.version)
                        .await?
                }
                None => Vec::new(),
            });
        }
        let mut planned = Vec::new();
        for (plan, progress) in live.iter().zip(&progress) {
            let Some(frozen) = &plan.live else { continue };
            for task in &frozen.tasks {
                let done = progress.iter().any(|p| p.task == task.number && p.done);
                if done {
                    continue;
                }
                planned.extend(task.elements.iter().map(|pin| PlannedPin {
                    workflow_id: frozen.workflow_id.clone(),
                    version: frozen.version,
                    task: task.number,
                    pin: pin.clone(),
                }));
            }
        }
        let states = self
            .code_record
            .element_states(project, &elements, &planned)
            .await?;
        Ok(Facts {
            stage,
            layout,
            elements,
            live,
            progress,
            states,
        })
    }

    /// A plan's line: its live version, its Draft, and its live tasks counted.
    fn plan_state(live: &LiveVersion, progress: &[TaskProgress]) -> PlanProgress {
        let mut tasks = TaskCounts::default();
        for task in progress {
            tasks.total += 1;
            if task.done {
                tasks.done += 1;
            } else if task.state.is_some() {
                tasks.started += 1;
            } else {
                tasks.not_started += 1;
            }
        }
        PlanProgress {
            plan_id: live.plan_id.clone(),
            title: live.title.clone(),
            workflow_id: live.live.as_ref().map(|f| f.workflow_id.clone()),
            version: live.live.as_ref().map(|f| f.version),
            draft: live.draft.clone(),
            tasks,
        }
    }

    /// The elements in gap or unchecked, by their part's map order, then element number.
    fn gaps(facts: &Facts) -> Vec<ElementGap> {
        let order: HashMap<&PartId, usize> = (facts.layout.parts.iter())
            .enumerate()
            .map(|(i, p)| (&p.id, i))
            .collect();
        let mut gaps: Vec<(usize, ElementGap)> = facts
            .elements
            .iter()
            .zip(&facts.states)
            .filter(|(_, s)| matches!(s.state, BuildState::Gap | BuildState::Unchecked))
            .map(|(element, state)| {
                let place = order.get(&element.part_id).copied().unwrap_or(usize::MAX);
                let gap = ElementGap {
                    element_id: element.id.clone(),
                    kind: element.content.kind(),
                    name: element.content.name().to_string(),
                    part_id: element.part_id.clone(),
                    state: state.state,
                    why: state.why.clone().unwrap_or_default(),
                };
                (place, gap)
            })
            .collect();
        // Stable: the elements already come by element number.
        gaps.sort_by_key(|(place, _)| *place);
        gaps.into_iter().map(|(_, gap)| gap).collect()
    }

    /// One part in full: its plans' live tasks with their progress, and its elements.
    fn detail(facts: &Facts, part: &PartId) -> PartDetail {
        let (plans, plans_more) = Self::bounded(
            facts
                .live
                .iter()
                .zip(&facts.progress)
                .filter(|(live, _)| live.parts.contains(part))
                .filter_map(|(live, progress)| {
                    let frozen = live.live.as_ref()?;
                    let tasks = frozen
                        .tasks
                        .iter()
                        .map(|task| {
                            let seen = progress.iter().find(|p| p.task == task.number);
                            TaskLine {
                                number: task.number,
                                title: task.title.clone(),
                                done: seen.is_some_and(|p| p.done),
                                attempt: seen.and_then(|p| p.state),
                            }
                        })
                        .collect();
                    Some(PlanTasks {
                        plan_id: live.plan_id.clone(),
                        title: live.title.clone(),
                        workflow_id: frozen.workflow_id.clone(),
                        version: frozen.version,
                        tasks,
                    })
                }),
            DETAIL_PLANS_MAX,
        );
        let elements = facts
            .elements
            .iter()
            .zip(&facts.states)
            .filter(|(element, _)| &element.part_id == part)
            .map(|(element, state)| ElementView {
                element: element.clone(),
                state: state.clone(),
            })
            .collect();
        PartDetail {
            part_id: part.clone(),
            plans,
            plans_more,
            elements,
        }
    }

    /// The first `max` items, and how many were left out.
    fn bounded<T>(items: impl Iterator<Item = T>, max: usize) -> (Vec<T>, u32) {
        let all: Vec<T> = items.collect();
        let more = Self::more(all.len(), max);
        (all.into_iter().take(max).collect(), more)
    }

    fn more(len: usize, max: usize) -> u32 {
        u32::try_from(len.saturating_sub(max)).unwrap_or(u32::MAX)
    }
}
