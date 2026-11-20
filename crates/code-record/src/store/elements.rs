//! The record's element pins.
use std::collections::BTreeMap;

use core_db::{Actor, JournalEntry, Storage, StorageError, Tx};
use core_plans::WorkflowId;
use core_projects::ProjectId;
use sqlx::Row;

use super::{CodeRecordStore, text};
use crate::{CodeRecordError, DoneRecord, ElementTask, SymbolState};

impl CodeRecordStore {
    pub(in crate::store) async fn write_elements_in(
        tx: &mut Tx,
        done: &DoneRecord,
    ) -> Result<(), StorageError> {
        for pin in &done.contract.elements {
            sqlx::query(
                "INSERT INTO code_record_element
                   (project_id, workflow_id, version, task, element_id, revision, action,
                    path, symbol_id)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(done.project.as_str())
            .bind(done.workflow.as_str())
            .bind(done.version)
            .bind(i64::from(done.task))
            .bind(&pin.id)
            .bind(pin.revision)
            .bind(text(&pin.action)?)
            .bind(&pin.file)
            .bind(&pin.symbol_id)
            .execute(&mut *tx)
            .await?;
        }
        Ok(())
    }

    pub(in crate::store) async fn clear_elements_in(
        tx: &mut Tx,
        workflow: &WorkflowId,
        version: i64,
        task: u32,
    ) -> Result<(), StorageError> {
        sqlx::query(
            "DELETE FROM code_record_element WHERE workflow_id = ? AND version = ? AND task = ?",
        )
        .bind(workflow.as_str())
        .bind(version)
        .bind(i64::from(task))
        .execute(&mut *tx)
        .await?;
        Ok(())
    }

    pub(in crate::store) async fn delete_element_project_in(
        tx: &mut Tx,
        project: &ProjectId,
    ) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM code_record_element WHERE project_id = ?")
            .bind(project.as_str())
            .execute(&mut *tx)
            .await?;
        Ok(())
    }

    pub(in crate::store) async fn element_change(
        tx: &mut Tx,
        project: &ProjectId,
        workflow: &WorkflowId,
        version: i64,
        task: u32,
    ) -> Result<(), StorageError> {
        let event = JournalEntry::new("CodeRecordChanged", Actor::system())
            .with_project(project.as_str())
            .with_payload(serde_json::json!({
                "project_id": project, "workflow_id": workflow, "version": version, "task": task
            }));
        JournalEntry::append_event(tx, event, &Storage::now()).await?;
        Ok(())
    }

    pub(super) async fn read_elements(
        &self,
        project: &ProjectId,
    ) -> Result<BTreeMap<String, Vec<ElementTask>>, CodeRecordError> {
        let rows = sqlx::query(
            "SELECT e.element_id, e.workflow_id, e.version, e.task, e.revision, e.action,
                    e.path, e.symbol_id, s.state
               FROM code_record_element e
               LEFT JOIN code_record_symbol s ON s.workflow_id = e.workflow_id
                AND s.version = e.version AND s.task = e.task AND s.symbol_id = e.symbol_id
              WHERE e.project_id = ?
              ORDER BY e.element_id, e.revision DESC, e.workflow_id, e.version, e.task",
        )
        .bind(project.as_str())
        .fetch_all(self.storage.reader())
        .await?;
        let mut elements: BTreeMap<String, Vec<ElementTask>> = BTreeMap::new();
        for row in rows {
            let action: String = row.try_get("action")?;
            let symbol_id: Option<String> = row.try_get("symbol_id")?;
            let state: Option<String> = row.try_get("state")?;
            let symbol_state = symbol_id
                .as_ref()
                .map(|_| {
                    state
                        .map(|state| serde_json::from_value(state.into()))
                        .transpose()
                        .map(|state| state.unwrap_or(SymbolState::Unchecked))
                })
                .transpose()?;
            elements
                .entry(row.try_get("element_id")?)
                .or_default()
                .push(ElementTask {
                    workflow_id: WorkflowId::from_recorded(row.try_get("workflow_id")?),
                    version: row.try_get("version")?,
                    task: row.try_get("task")?,
                    revision: row.try_get("revision")?,
                    action: serde_json::from_value(action.into())?,
                    file: row.try_get("path")?,
                    symbol_id,
                    symbol_state,
                });
        }
        Ok(elements)
    }
}
