//! The person's record: their names, language and theme, independent of any agent.
mod model;
mod store;

pub use model::{Person, PersonInput, Theme};

mod error;
use core_db::{CommandContext, Storage};
pub use error::PersonError;
use std::sync::Arc;
use store::PersonStore;

#[derive(Clone)]
pub struct PersonSettings {
    store: PersonStore,
}

impl PersonSettings {
    pub fn new(storage: Arc<Storage>) -> Self {
        Self {
            store: PersonStore::new(storage),
        }
    }

    pub async fn person(&self) -> Result<Person, PersonError> {
        Ok(self.store.get().await?)
    }

    pub async fn set_person(
        &self,
        command_id: String,
        mut input: PersonInput,
    ) -> Result<Person, PersonError> {
        normalize_name(&mut input.name)?;
        normalize_name(&mut input.aylul_calls_me)?;
        if input
            .language
            .as_deref()
            .is_some_and(|tag| !language_tag(tag))
        {
            return Err(PersonError::InvalidLanguage);
        }
        let command = CommandContext::user(command_id, "person.set", serde_json::json!(input));
        self.store.save(command, input).await?;
        self.person().await
    }
}

fn normalize_name(value: &mut Option<String>) -> Result<(), PersonError> {
    let Some(name) = value.take() else {
        return Ok(());
    };
    if name
        .chars()
        .any(|c| c.is_control() || matches!(c as u32, 0x2028 | 0x2029))
    {
        return Err(PersonError::InvalidName);
    }
    let name = name.trim();
    if name.chars().count() > 64 {
        return Err(PersonError::NameTooLong);
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
