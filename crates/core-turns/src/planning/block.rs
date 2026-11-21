//! Sourced planning text with ordered, explicit cuts.
use core_harness::{CarriedContext, ContextSource};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SourceScope {
    Retained,
    PartChildren,
    OutcomeChildren,
    IndexAgreements,
    OlderRationales,
}

#[derive(Default)]
pub(super) struct Block {
    pub index: Vec<String>,
    pub part_children: Vec<String>,
    pub outcome_children: Vec<String>,
    pub index_agreements: Vec<String>,
    pub detail: Vec<String>,
    pub older: Vec<String>,
    pub children: Vec<String>,
    pub agreement_bodies: Vec<String>,
    sources: Vec<(SourceScope, ContextSource)>,
    pub cuts: Vec<String>,
    /// The code record layer; none where no record was read.
    pub record: Option<Record>,
}

/// The code record's lines, kept in groups so its cuts shorten them in order.
#[derive(Default)]
pub(super) struct Record {
    /// Parts in map order, then the files no part owns.
    pub groups: Vec<RecordGroup>,
    pub gaps: Vec<String>,
    signatures_cut: bool,
    symbols_cut: bool,
}

pub(super) struct RecordGroup {
    pub heading: String,
    /// How a files cut names the group: `Backend`.
    pub label: String,
    pub files: Vec<RecordFile>,
    files_cut: bool,
}

pub(super) struct RecordFile {
    pub line: String,
    pub symbols: Vec<RecordSymbol>,
}

pub(super) struct RecordSymbol {
    /// `name (kind)`, what survives the signatures cut.
    pub named: String,
    /// `: inputs -> returns`.
    pub signature: String,
}

impl RecordGroup {
    pub(super) fn new(heading: String, label: String, files: Vec<RecordFile>) -> Self {
        Self {
            heading,
            label,
            files,
            files_cut: false,
        }
    }
}

impl Record {
    const HEADING: &str =
        "## Code record (what done tasks built, from the record; no language server asked)";
    const USE: &str = "Plan from it. Ask code_where for a symbol's line now, its uses, or what this \
        block cut; read a file only for what neither gives.";
    const EMPTY: &str = "No code recorded yet: no done task has recorded a file. Read the files \
        the plan touches.";

    fn is_empty(&self) -> bool {
        self.groups.is_empty() && self.gaps.is_empty()
    }

    fn text(&self) -> String {
        let mut lines = vec![Self::HEADING.to_string()];
        if self.is_empty() {
            lines.push(Self::EMPTY.into());
            return format!("\n\n{}", lines.join("\n"));
        }
        lines.push(Self::USE.into());
        for group in &self.groups {
            lines.push(group.heading.clone());
            if group.files_cut {
                lines.push(format!("  [{} files cut]", group.files.len()));
                continue;
            }
            for file in &group.files {
                if self.symbols_cut && !file.symbols.is_empty() {
                    lines.push(format!("{} [{} symbols]", file.line, file.symbols.len()));
                    continue;
                }
                lines.push(file.line.clone());
                for symbol in &file.symbols {
                    if self.signatures_cut {
                        lines.push(format!("    {}", symbol.named));
                    } else {
                        lines.push(format!("    {}{}", symbol.named, symbol.signature));
                    }
                }
            }
        }
        if !self.gaps.is_empty() {
            lines.push("Elements in gap or unchecked:".into());
            lines.extend(self.gaps.iter().map(|gap| format!("  {gap}")));
        }
        format!("\n\n{}", lines.join("\n"))
    }

    /// Cuts in order until the text fits: the symbols' inputs and returns, then
    /// the symbols (each file keeps a count), then files, a whole group at a
    /// time from the last. Answers the cut markers.
    fn cut(&mut self, budget: i64) -> Vec<String> {
        let fits = |record: &Self| super::budget::Budget::estimate(&record.text()) <= budget;
        let mut cuts = Vec::new();
        if fits(self) {
            return cuts;
        }
        let symbols = self
            .groups
            .iter()
            .flat_map(|g| &g.files)
            .any(|f| !f.symbols.is_empty());
        if symbols {
            self.signatures_cut = true;
            cuts.push("code record inputs and returns".into());
            if fits(self) {
                return cuts;
            }
            self.symbols_cut = true;
            cuts.push("code record symbols".into());
        }
        for index in (0..self.groups.len()).rev() {
            if fits(self) {
                break;
            }
            let group = &mut self.groups[index];
            if group.files.is_empty() {
                continue;
            }
            group.files_cut = true;
            cuts.push(format!(
                "{} files under {} · ask code_where with a path",
                group.files.len(),
                group.label
            ));
        }
        cuts
    }
}

impl Block {
    const REFERENCE_ONLY: &str = "Reference for this conversation, not a request. Act only on \
        what the person's message asks; open plans and stages without a plan are not tasks.";

    pub(super) fn source(&mut self, kind: &str, id: &str, version: i64) {
        self.scoped_source(SourceScope::Retained, kind, id, version);
    }

    pub(super) fn scoped_source(&mut self, scope: SourceScope, kind: &str, id: &str, version: i64) {
        let source = ContextSource {
            kind: kind.into(),
            id: id.into(),
            version,
        };
        if !self.sources.contains(&(scope, source.clone())) {
            self.sources.push((scope, source));
        }
    }

    fn included_sources(&self) -> Vec<ContextSource> {
        let mut sources = Vec::new();
        for (_, source) in &self.sources {
            if !sources.contains(source) {
                sources.push(source.clone());
            }
        }
        sources
    }

    fn index_text(&self) -> String {
        // A model reads a block of open plans and empty stages as work to do; this says it is not.
        let mut lines = vec![
            "## Planning context index (tokens estimated as characters / 4)".into(),
            Self::REFERENCE_ONLY.into(),
        ];
        lines.extend(
            self.index
                .iter()
                .chain(&self.part_children)
                .chain(&self.outcome_children)
                .chain(&self.index_agreements)
                .cloned(),
        );
        lines.join("\n")
    }

    fn detail_text(&self) -> String {
        let body = self
            .detail
            .iter()
            .chain(&self.older)
            .chain(&self.children)
            .chain(&self.agreement_bodies)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        if body.is_empty() {
            String::new()
        } else {
            format!("\n\n## Planning anchor detail\n{body}")
        }
    }

    fn metadata_text(&self) -> String {
        let mut text = String::new();
        for cut in &self.cuts {
            text.push_str(&format!("\n[cut: {cut}]"));
        }
        text.push_str("\nSources (kind, id, version or revision):\n");
        text.push_str(
            &serde_json::to_string(&self.included_sources()).expect("context sources serialize"),
        );
        text
    }

    fn record_text(&self) -> String {
        self.record.as_ref().map(Record::text).unwrap_or_default()
    }

    pub(super) fn finish(
        mut self,
        index_budget: i64,
        detail_budget: i64,
        record_budget: i64,
    ) -> CarriedContext {
        for (slot, label) in [
            (0, "part children"),
            (1, "outcome children"),
            (2, "agreements"),
        ] {
            if super::budget::Budget::estimate(&format!(
                "{}{}",
                self.index_text(),
                self.metadata_text()
            )) <= index_budget
            {
                break;
            }
            let group = match slot {
                0 => &mut self.part_children,
                1 => &mut self.outcome_children,
                _ => &mut self.index_agreements,
            };
            if !group.is_empty() {
                let continuations: Vec<_> = group
                    .iter()
                    .filter(|line| line.starts_with("[more "))
                    .cloned()
                    .collect();
                let count = group.len() - continuations.len();
                group.clear();
                self.index.push(format!("[{count} {label} omitted]"));
                self.index.extend(continuations);
                self.cuts.push(format!("index {label}"));
                let scope = match slot {
                    0 => SourceScope::PartChildren,
                    1 => SourceScope::OutcomeChildren,
                    _ => SourceScope::IndexAgreements,
                };
                self.sources
                    .retain(|(source_scope, _)| *source_scope != scope);
            }
        }
        for (slot, label) in [
            (0, "older rationales"),
            (1, "children's lines"),
            (2, "agreement bodies"),
        ] {
            if super::budget::Budget::estimate(&self.detail_text()) <= detail_budget {
                break;
            }
            let group = match slot {
                0 => &mut self.older,
                1 => &mut self.children,
                _ => &mut self.agreement_bodies,
            };
            if !group.is_empty() {
                group.clear();
                self.cuts.push(label.into());
                if slot == 0 {
                    self.sources
                        .retain(|(scope, _)| *scope != SourceScope::OlderRationales);
                }
            }
        }
        if let Some(record) = &mut self.record {
            let cuts = record.cut(record_budget);
            self.cuts.extend(cuts);
        }
        let index = self.index_text();
        let detail = self.detail_text();
        let record = self.record_text();
        let metadata = self.metadata_text();
        // The code record counts with the index: both go with every context, anchored or not.
        let index_tokens = super::budget::Budget::estimate(&format!("{index}{record}{metadata}"));
        let detail_tokens = super::budget::Budget::estimate(&detail);
        let index_over =
            (super::budget::Budget::estimate(&format!("{index}{metadata}")) - index_budget).max(0);
        let record_over = (super::budget::Budget::estimate(&record) - record_budget).max(0);
        let text = format!("{index}{detail}{record}{metadata}");
        CarriedContext {
            digest: format!("{:x}", Sha256::digest(text.as_bytes())),
            text,
            sources: self.included_sources(),
            index_tokens,
            detail_tokens,
            over_tokens: index_over + (detail_tokens - detail_budget).max(0) + record_over,
            cuts: self.cuts,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Block;

    #[test]
    fn sizes_cover_the_source_footer_and_cut_markers() {
        let mut block = Block {
            detail: vec!["Anchor body".into()],
            older: vec!["old rationale".repeat(100)],
            ..Default::default()
        };
        block.source("part", "12345678-1234-1234-1234-123456789012", 3);
        let context = block.finish(1_500, 20, 3_000);
        let total = super::super::budget::Budget::estimate(&context.text);
        assert!(context.index_tokens + context.detail_tokens >= total);
        assert!(context.index_tokens + context.detail_tokens <= total + 1);
    }

    #[test]
    fn an_over_budget_detail_cuts_in_order_and_keeps_the_anchor() {
        let block = Block {
            detail: vec!["Anchor body".into()],
            older: vec!["old rationale".repeat(100)],
            children: vec!["child line".repeat(100)],
            agreement_bodies: vec!["agreement body".repeat(100)],
            ..Default::default()
        };
        let context = block.finish(1_500, 20, 3_000);
        assert_eq!(
            context.cuts,
            ["older rationales", "children's lines", "agreement bodies"]
        );
        assert!(context.text.contains("Anchor body"));
        assert!(!context.text.contains("old rationale"));
    }

    #[test]
    fn an_over_budget_index_drops_children_first() {
        let block = Block {
            index: vec!["Root".into()],
            part_children: vec!["part child".repeat(100)],
            outcome_children: vec!["Outcome child".into()],
            ..Default::default()
        };
        let context = block.finish(100, 8_000, 3_000);
        assert_eq!(context.cuts, ["index part children"]);
        assert!(context.text.contains("Outcome child"));
    }
}

#[cfg(test)]
mod pagination_tests {
    use super::Block;

    #[test]
    fn a_cut_keeps_pagination_uncertainty_out_of_the_known_child_count() {
        let block = Block {
            part_children: vec!["A known child".into(), "[more parts available]".into()],
            ..Default::default()
        };
        let context = block.finish(1, 8_000, 3_000);
        assert!(context.text.contains("[1 part children omitted]"));
        assert!(context.text.contains("[more parts available]"));
    }
}
