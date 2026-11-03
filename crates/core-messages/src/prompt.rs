use crate::{Group, Message, Messages, Speaker};

pub(super) const MAX_MESSAGES: i64 = 200;
const MAX_CHARS: usize = 32_000;

impl Messages {
    pub(super) fn delivery_prompt(group: &Group, messages: &[Message], total: i64) -> String {
        let members = group
            .members
            .iter()
            .map(|member| format!("{} (@{})", member.name, member.slug))
            .collect::<Vec<_>>()
            .join(", ");
        let mut prompt = format!(
            "Group: {}\nMembers: {members}\n\
         Answer in 1-3 sentences. Mention @slug only when you need that member, \
         or @mohammed when you need Mohammed.\n\n",
            group.name,
        );
        let mut remaining = MAX_CHARS.saturating_sub(prompt.chars().count() + 80);
        let mut lines = Vec::new();
        for message in messages.iter().rev() {
            let speaker = match &message.speaker {
                Speaker::Person => "Mohammed",
                Speaker::Agent { slug, .. } => slug,
                Speaker::Shadows => "Shadows",
            };
            let mut line = format!("{speaker}: {}\n", message.body);
            let size = line.chars().count();
            if size > remaining {
                if lines.is_empty() {
                    line = line.chars().take(remaining.saturating_sub(2)).collect();
                    line.push_str("…\n");
                    lines.push(line);
                }
                break;
            }
            remaining -= size;
            lines.push(line);
        }
        let omitted = total - lines.len() as i64;
        if omitted > 0 {
            prompt.push_str(&format!("[{omitted} earlier messages omitted]\n"));
        }
        for line in lines.into_iter().rev() {
            prompt.push_str(&line);
        }
        prompt
    }
}

#[cfg(test)]
mod tests {
    use super::MAX_CHARS;
    use crate::{Group, GroupId, Message, MessageId, MessageKind, Messages, Speaker};

    #[test]
    fn the_trigger_survives_the_cut() {
        let group = Group {
            id: GroupId::generate(),
            name: "Test".into(),
            members: Vec::new(),
            needs_you: false,
            needs_you_since: None,
            created_at: String::new(),
        };
        let messages: Vec<_> = (1..=200)
            .map(|seq| Message {
                id: MessageId::generate(),
                group_id: group.id.clone(),
                seq,
                speaker: Speaker::Person,
                to: Vec::new(),
                kind: MessageKind::Chat,
                body: format!("line {seq} {}", "x".repeat(8000)),
                created_at: String::new(),
            })
            .collect();
        let prompt = Messages::delivery_prompt(&group, &messages, 300);
        assert!(prompt.contains("[297 earlier messages omitted]"));
        assert!(prompt.contains("line 200"));
        assert!(!prompt.contains("line 197"));
        assert!(prompt.chars().count() <= MAX_CHARS);
    }
}
