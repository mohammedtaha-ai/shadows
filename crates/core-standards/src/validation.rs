//! Validation of project additions against the mandatory base parts.

use super::{Standards, StandardsAdditions};

impl Standards {
    pub(super) fn validate(additions: &StandardsAdditions) -> Result<(), String> {
        let mut names: Vec<_> = Self::base().parts.iter().map(|p| p.name.as_str()).collect();
        for part in &additions.parts {
            let name = part.name.as_str();
            let shaped = name.starts_with(|c: char| c.is_ascii_lowercase())
                && name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
            if !shaped {
                return Err(format!(
                    "part \"{name}\": use lowercase letters, digits and -"
                ));
            }
            if names.contains(&name) {
                return Err(format!("part \"{name}\" is already a mandatory part"));
            }
            if part.owns.trim().is_empty() {
                return Err(format!("part \"{name}\" needs what it owns"));
            }
            names.push(name);
        }
        let mut ids = Vec::new();
        for rule in &additions.rules {
            let id = rule.id.as_str();
            let numbered = id.strip_prefix('P').is_some_and(|n| {
                n.starts_with(|c: char| c.is_ascii_digit() && c != '0')
                    && n.bytes().all(|c| c.is_ascii_digit())
            });
            if !numbered {
                return Err(format!("rule \"{id}\": a project rule's id is P1, P2, ..."));
            }
            if ids.contains(&id) {
                return Err(format!("rule {id} appears twice"));
            }
            if rule.text.trim().is_empty() {
                return Err(format!("rule {id} has no text"));
            }
            if let Some(unknown) = rule.parts.iter().find(|p| !names.contains(&p.as_str())) {
                return Err(format!(
                    "rule {id} names \"{unknown}\", which is not a part"
                ));
            }
            ids.push(id);
        }
        Ok(())
    }
}
