//! Judging a design element's build state.
use std::collections::{BTreeMap, BTreeSet};

use codebase::Asker;
use core_design::DesignElement;
use core_plans::ElementAction;
use core_projects::ProjectId;

use crate::{
    BuildState, CodeRecord, CodeRecordError, Confirmation, ElementState, ElementTask, PlannedPin,
    SymbolState,
};

impl CodeRecord {
    /// Resolves each element against the record, planned pins and the project folder.
    pub async fn element_states(
        &self,
        project: &ProjectId,
        elements: &[DesignElement],
        planned: &[PlannedPin],
    ) -> Result<Vec<ElementState>, CodeRecordError> {
        let mut done = self.store.element_pins(project).await?;
        for pins in done.values_mut() {
            if let Some(highest) = pins.iter().map(|pin| pin.revision).max() {
                pins.retain(|pin| pin.revision == highest);
            }
        }
        let files: Vec<String> = elements
            .iter()
            .filter_map(|element| done.get(&element.id))
            .flatten()
            .filter(|pin| pin.action != ElementAction::Delete)
            .map(|pin| pin.file.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let (absent, folder_error) = if files.is_empty() {
            (BTreeSet::new(), None)
        } else {
            match self
                .code
                .missing_inputs(Asker::Person(project), &files)
                .await
            {
                Ok(absent) => (absent.into_iter().collect(), None),
                Err(error) => (
                    BTreeSet::new(),
                    Some(format!("the folder could not be read: {error}")),
                ),
            }
        };
        let mut upcoming: BTreeMap<&str, Vec<ElementTask>> = BTreeMap::new();
        for PlannedPin {
            workflow_id,
            version,
            task,
            pin,
        } in planned
        {
            upcoming.entry(&pin.id).or_default().push(ElementTask {
                workflow_id: workflow_id.clone(),
                version: *version,
                task: *task,
                revision: pin.revision,
                action: pin.action,
                file: pin.file.clone(),
                symbol_id: pin.symbol_id.clone(),
                symbol_state: None,
            });
        }
        Ok(elements
            .iter()
            .map(|element| {
                judge(
                    element,
                    done.get(&element.id).map(Vec::as_slice).unwrap_or_default(),
                    upcoming.remove(element.id.as_str()).unwrap_or_default(),
                    &absent,
                    folder_error.as_deref(),
                )
            })
            .collect())
    }
}

fn judge(
    latest: &DesignElement,
    done: &[ElementTask],
    planned: Vec<ElementTask>,
    absent: &BTreeSet<String>,
    folder_error: Option<&str>,
) -> ElementState {
    let built_revision = done.first().map(|pin| pin.revision);
    let behind = built_revision.is_some_and(|revision| revision < latest.revision);
    let removed = latest.deleted
        && (done.is_empty()
            || (!behind && done.iter().any(|pin| pin.action == ElementAction::Delete)));
    let (state, why) = if removed {
        (BuildState::Removed, None)
    } else if done.is_empty() {
        (BuildState::Planned, None)
    } else {
        outcome(done, absent, folder_error)
    };
    let confirmed = (state == BuildState::Built).then(|| {
        if done.iter().all(|pin| pin.symbol_id.is_some()) {
            Confirmation::FileAndSymbol
        } else {
            Confirmation::FileOnly
        }
    });
    ElementState {
        element_id: latest.id.clone(),
        state,
        built_revision,
        behind,
        confirmed,
        done: done.to_vec(),
        planned,
        why,
    }
}

fn outcome(
    done: &[ElementTask],
    absent: &BTreeSet<String>,
    folder_error: Option<&str>,
) -> (BuildState, Option<String>) {
    if let Some(pin) = done
        .iter()
        .find(|pin| pin.action != ElementAction::Delete && absent.contains(&pin.file))
    {
        return (
            BuildState::Gap,
            Some(format!(
                "`{}` is absent; T{} of {} v{} built it",
                pin.file, pin.task, pin.workflow_id, pin.version
            )),
        );
    }
    for pin in done {
        if let (Some(symbol), Some(SymbolState::Missing)) = (&pin.symbol_id, pin.symbol_state) {
            return (
                BuildState::Gap,
                Some(format!(
                    "{symbol} in {} was missing when T{} was marked done",
                    pin.file, pin.task
                )),
            );
        }
    }
    if let Some(reason) = folder_error
        && done.iter().any(|pin| pin.action != ElementAction::Delete)
    {
        return (BuildState::Unchecked, Some(reason.into()));
    }
    for pin in done {
        if let (Some(symbol), Some(SymbolState::Unchecked)) = (&pin.symbol_id, pin.symbol_state) {
            return (
                BuildState::Unchecked,
                Some(format!(
                    "no language server confirmed {symbol} in {}",
                    pin.file
                )),
            );
        }
    }
    (BuildState::Built, None)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use core_plans::{ElementAction, WorkflowId};

    use super::outcome;
    use crate::{BuildState, ElementTask, SymbolState};

    fn pin(state: SymbolState) -> ElementTask {
        ElementTask {
            workflow_id: WorkflowId::from_recorded("workflow".into()),
            version: 1,
            task: 1,
            revision: 1,
            action: ElementAction::Create,
            file: "users.rs".into(),
            symbol_id: Some("s1".into()),
            symbol_state: Some(state),
        }
    }

    #[test]
    fn an_absent_file_is_a_gap_before_an_unchecked_symbol() {
        let pins = [pin(SymbolState::Unchecked)];
        let absent = BTreeSet::from(["users.rs".into()]);
        let (state, why) = outcome(&pins, &absent, None);
        assert_eq!(state, BuildState::Gap);
        assert!(why.unwrap().contains("is absent"));
    }

    #[test]
    fn a_missing_symbol_is_a_gap() {
        let pins = [pin(SymbolState::Missing)];
        let (state, why) = outcome(&pins, &BTreeSet::new(), Some("unreadable"));
        assert_eq!(state, BuildState::Gap);
        assert!(why.unwrap().contains("s1 in users.rs was missing"));
    }

    #[test]
    fn an_unchecked_symbol_is_unchecked_and_a_found_one_built() {
        let absent = BTreeSet::new();
        let (unchecked, _) = outcome(&[pin(SymbolState::Unchecked)], &absent, None);
        let (built, why) = outcome(&[pin(SymbolState::Found)], &absent, None);
        assert_eq!(unchecked, BuildState::Unchecked);
        assert_eq!((built, why), (BuildState::Built, None));
    }

    #[test]
    fn a_folder_that_cannot_be_read_makes_the_element_unchecked_with_the_reason() {
        let pins = [pin(SymbolState::Found)];
        let (state, why) = outcome(&pins, &BTreeSet::new(), Some("not readable"));
        assert_eq!(
            (state, why.as_deref()),
            (BuildState::Unchecked, Some("not readable"))
        );
    }
}
