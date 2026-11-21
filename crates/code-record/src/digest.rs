//! One job: the digest of the record — what done tasks built, by part and
//! layer, read from the record alone for a Planner's context.

use std::collections::BTreeMap;

use core_design::ProjectLayout;
use core_plans::SymbolWork;
use core_projects::ProjectId;

use crate::store::Detail;
use crate::{
    BuildState, CodeRecord, CodeRecordError, DigestFile, DigestGap, DigestPart, DigestSymbol,
    RecordDigest,
};

impl CodeRecord {
    /// What `project`'s done tasks built: per part of the approved map in map
    /// order, each recorded file with its purpose and its symbols' signatures;
    /// the files no part owns; the elements in gap or unchecked, with why; and
    /// the count of done marks. No language server is asked.
    pub async fn digest(&self, project: &ProjectId) -> Result<RecordDigest, CodeRecordError> {
        let (detail, marks) = self.store.whole(project).await?;
        let layout = self.design.layout(project).await?;
        let elements = self.design.elements(project).await?;
        let states = self.element_states(project, &elements, &[]).await?;
        let mut gaps: Vec<(usize, DigestGap)> = elements
            .iter()
            .zip(&states)
            .filter(|(_, s)| matches!(s.state, BuildState::Gap | BuildState::Unchecked))
            .map(|(element, state)| {
                let place = (layout.parts.iter())
                    .position(|p| p.id == element.part_id)
                    .unwrap_or(usize::MAX);
                let gap = DigestGap {
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
        // Stable: Design answers the elements by element number.
        gaps.sort_by_key(|(place, _)| *place);
        let (parts, unowned) = Self::by_part(&layout, Self::files(&detail)?);
        Ok(RecordDigest {
            parts,
            unowned,
            gaps: gaps.into_iter().map(|(_, gap)| gap).collect(),
            marks,
        })
    }

    /// Each recorded path once, with its newest record's purpose and the
    /// newest record of each of its symbols, by name. `detail` comes by path,
    /// newest first.
    fn files(detail: &Detail) -> Result<Vec<DigestFile>, CodeRecordError> {
        let mut files: Vec<DigestFile> = Vec::new();
        for row in &detail.files {
            if files.last().is_none_or(|f| f.path != row.path) {
                files.push(DigestFile {
                    path: row.path.clone(),
                    layer: None,
                    purpose: row.purpose.clone(),
                    symbols: Vec::new(),
                });
            }
        }
        let mut symbols: BTreeMap<(&str, &str), DigestSymbol> = BTreeMap::new();
        for row in &detail.symbols {
            if symbols.contains_key(&(row.path.as_str(), row.name.as_str())) {
                continue;
            }
            // The row holds the contract's symbol as its task wrote it.
            let work: SymbolWork = serde_json::from_str(&row.contract_json)?;
            symbols.insert(
                (row.path.as_str(), row.name.as_str()),
                DigestSymbol {
                    name: work.name,
                    kind: work.kind,
                    inputs: work.inputs,
                    returns: work.returns,
                },
            );
        }
        for ((path, _), symbol) in symbols {
            if let Some(file) = files.iter_mut().find(|f| f.path == path) {
                file.symbols.push(symbol);
            }
        }
        Ok(files)
    }

    /// Files under the part whose folder holds them, in map order, then by the
    /// layer's place in its part (no layer last) and path; the rest by path.
    fn by_part(
        layout: &ProjectLayout,
        files: Vec<DigestFile>,
    ) -> (Vec<DigestPart>, Vec<DigestFile>) {
        let mut parts: Vec<DigestPart> = layout
            .parts
            .iter()
            .map(|p| DigestPart {
                part_id: p.id.clone(),
                title: p.title.clone(),
                files: Vec::new(),
            })
            .collect();
        let mut unowned = Vec::new();
        for mut file in files {
            let Some(owner) = Self::path_owner(layout, &file.path) else {
                unowned.push(file);
                continue;
            };
            file.layer = owner.layer;
            match parts.iter_mut().find(|p| p.part_id == owner.part_id) {
                Some(part) => part.files.push(file),
                None => unowned.push(file),
            }
        }
        for (part, laid) in parts.iter_mut().zip(&layout.parts) {
            let place = |layer: &Option<String>| {
                layer
                    .as_ref()
                    .and_then(|id| laid.layers.iter().position(|l| &l.id == id))
                    .unwrap_or(usize::MAX)
            };
            part.files
                .sort_by(|a, b| (place(&a.layer), &a.path).cmp(&(place(&b.layer), &b.path)));
        }
        parts.retain(|p| !p.files.is_empty());
        (parts, unowned)
    }
}
