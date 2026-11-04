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
}

impl Block {
    pub fn source(&mut self, kind: &str, id: &str, version: i64) {
        self.scoped_source(SourceScope::Retained, kind, id, version);
    }

    pub fn scoped_source(&mut self, scope: SourceScope, kind: &str, id: &str, version: i64) {
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
        let mut lines =
            vec!["## Planning context index (tokens estimated as characters / 4)".into()];
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

    pub fn finish(mut self, index_budget: i64, detail_budget: i64) -> CarriedContext {
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
        let index = self.index_text();
        let detail = self.detail_text();
        let metadata = self.metadata_text();
        let index_tokens = super::budget::Budget::estimate(&format!("{index}{metadata}"));
        let detail_tokens = super::budget::Budget::estimate(&detail);
        let text = format!("{index}{detail}{metadata}");
        CarriedContext {
            digest: format!("{:x}", Sha256::digest(text.as_bytes())),
            text,
            sources: self.included_sources(),
            index_tokens,
            detail_tokens,
            over_tokens: (index_tokens - index_budget).max(0)
                + (detail_tokens - detail_budget).max(0),
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
        let context = block.finish(1_500, 20);
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
        let context = block.finish(1_500, 20);
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
        let context = block.finish(100, 8_000);
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
        let context = block.finish(1, 8_000);
        assert!(context.text.contains("[1 part children omitted]"));
        assert!(context.text.contains("[more parts available]"));
    }
}
