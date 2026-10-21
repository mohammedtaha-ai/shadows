//! The person's record rules, independent of an account or agent home.
use super::{Person, PersonInput, Profiles, store};
use crate::CoreError;
use crate::db::CommandContext;

fn normalize_name(value: &mut Option<String>) -> Result<(), CoreError> {
    let Some(name) = value.take() else {
        return Ok(());
    };
    if name
        .chars()
        .any(|c| c.is_control() || matches!(c as u32, 0x2028 | 0x2029))
    {
        return Err(CoreError::ProfileInvalid("invalid person name".into()));
    }
    let name = name.trim();
    if name.chars().count() > 64 {
        return Err(CoreError::ProfileInvalid("person name is too long".into()));
    }
    *value = (!name.is_empty()).then(|| name.to_owned());
    Ok(())
}

fn language_tag(tag: &str) -> bool {
    let (base, suffix) = tag
        .split_once('-')
        .map_or((tag, None), |(a, b)| (a, Some(b)));
    (2..=3).contains(&base.len())
        && base.bytes().all(|b| b.is_ascii_lowercase())
        && suffix.is_none_or(|s| {
            (2..=8).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphanumeric())
        })
}

impl Profiles {
    pub async fn person(&self) -> Result<Person, CoreError> {
        Ok(store::person(&self.storage).await?)
    }

    pub async fn set_person(
        &self,
        command_id: String,
        mut input: PersonInput,
    ) -> Result<Person, CoreError> {
        normalize_name(&mut input.name)?;
        normalize_name(&mut input.aylul_calls_me)?;
        if input
            .language
            .as_deref()
            .is_some_and(|tag| !language_tag(tag))
        {
            return Err(CoreError::ProfileInvalid("invalid language tag".into()));
        }
        let command = CommandContext::user(command_id, "person.set", serde_json::json!(input));
        store::save_person(&self.storage, command, input).await?;
        self.person().await
    }
}
