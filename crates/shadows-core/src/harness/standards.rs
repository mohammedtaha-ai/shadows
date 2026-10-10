//! One job: rendering effective standards for the Planner (§23.4).

use crate::design::EffectiveStandards;

pub(super) fn render(s: &EffectiveStandards) -> String {
    let additions = s.additions.as_ref();
    let mut out = format!(
        "## Shadows standards (base v{}; project additions {})\n\nMandatory parts:\n",
        s.base.version,
        additions.map_or("none".to_string(), |a| format!("v{}", a.number)),
    );
    for p in &s.base.parts {
        let waiver = if p.waivable {
            "waivable with a reason"
        } else {
            "never waived"
        };
        out.push_str(&format!("- {} ({waiver}): {}\n", p.name, p.owns));
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
    out.push_str("\nContract template. Rules:\n");
    for r in &s.base.contract_template.rules {
        out.push_str(&format!("- {r}\n"));
    }
    out.push_str(&format!(
        "Sections, in order: {}\n",
        s.base.contract_template.shape.join(", ")
    ));
    out
}
