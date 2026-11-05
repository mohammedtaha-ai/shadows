//! One job: rendering effective standards for the Planner.

use core_standards::{EffectiveStandards, Presence};

impl super::Harness {
    pub(super) fn render(s: &EffectiveStandards) -> String {
        let additions = s.additions.as_ref();
        let mut out = format!(
            "## Shadows standards (base v{}; project additions {})\n\nParts:\n",
            s.base.version,
            additions.map_or("none".to_string(), |a| format!("v{}", a.number)),
        );
        for p in &s.base.parts {
            let presence = match p.presence {
                Presence::Required => "required",
                Presence::Waivable => "waivable with a reason",
                Presence::Optional => "optional",
            };
            out.push_str(&format!("- {} ({presence}): {}\n", p.name, p.owns));
        }
        for p in additions.iter().flat_map(|a| &a.content.parts) {
            out.push_str(&format!("- {} (project): {}\n", p.name, p.owns));
        }
        out.push_str("\nRules:\n");
        let rules = s
            .base
            .rules
            .iter()
            .chain(additions.iter().flat_map(|a| &a.content.rules));
        for r in rules {
            let parts = if r.parts.is_empty() {
                "all".into()
            } else {
                r.parts.join(", ")
            };
            out.push_str(&format!("- {} [{parts}]: {}\n", r.id, r.text));
        }
        out
    }
}
