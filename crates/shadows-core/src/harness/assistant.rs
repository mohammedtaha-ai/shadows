//! Opening an assistant on the agent's provider configuration.
use super::sessions::{OpenError, Sessions};
use crate::{CoreError, threads::TurnContext};
use shadows_agent::{acp::SessionSetup, policy, profile::ClaudeProfile};
use shadows_process::ProcessSpec;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AylulConfig {
    pub home_root: PathBuf,
    pub project_slug: String,
}

impl Sessions {
    fn aylul_setup() -> SessionSetup {
        SessionSetup {
            mcp: None,
            append: None,
            allowed_tools: vec![],
            tools: None,
            setting_sources: Some(vec!["user".into()]),
        }
    }
}

pub(super) struct Opening {
    pub(super) cwd: PathBuf,
    pub(super) setup: SessionSetup,
    pub(super) spec: ProcessSpec,
    pub(super) aylul_usage: Option<super::AylulUsageSnapshot>,
    pub(super) aylul_models: Option<(Vec<String>, String)>,
}

impl Sessions {
    pub(crate) async fn harness_available(&self, kind: &str) -> Result<bool, CoreError> {
        let configured =
            self.config.aylul.is_some() && !self.owners.agents.agents().await?.is_empty();
        Ok(policy::is_available_with(kind, configured))
    }

    pub(crate) async fn check_context(&self, context: &TurnContext) -> Result<(), CoreError> {
        if context.harness == policy::AYLUL {
            let slug = context
                .profile
                .as_deref()
                .ok_or_else(|| CoreError::AylulProfileNotFound("agent required".into()))?;
            let agent = self.owners.agents.agent(slug).await?;
            if agent.deleted {
                return Err(CoreError::AylulProfileNotFound(slug.into()));
            }
            self.owners.providers.provider(&agent.provider).await?;
        }
        Ok(())
    }

    pub(super) async fn opening(&self, context: &TurnContext) -> Result<Opening, OpenError> {
        if context.harness != policy::AYLUL {
            let cwd = workspace(context).map_err(OpenError::Workspace)?;
            let setup = SessionSetup::default();
            return Ok(Opening {
                spec: self.adapter.process_spec(&cwd),
                cwd,
                setup,
                aylul_usage: None,
                aylul_models: None,
            });
        }
        self.check_context(context).await?;
        let config = self
            .config
            .aylul
            .as_ref()
            .ok_or(CoreError::AylulNotConfigured)?;
        let slug = context
            .profile
            .as_deref()
            .ok_or(CoreError::AylulNotConfigured)?;
        let agent = self.owners.agents.agent(slug).await?;
        let provider = self.owners.providers.provider(&agent.provider).await?;
        let home = self.owners.agents.home(slug)?;
        let models: Vec<_> = provider.models.iter().map(|m| m.id.clone()).collect();
        let window = provider
            .models
            .iter()
            .find(|m| m.id == agent.default_model)
            .ok_or_else(|| CoreError::ProfileInvalid("default model is not offered".into()))?
            .context_window;
        self.homes
            .write_extensions(&self.owners.extensions, &config.home_root, &home, slug)
            .await?;
        self.homes
            .write_models(&home, &models, &agent.default_model)
            .await?;
        let secrets = self
            .config
            .secrets
            .as_ref()
            .ok_or(CoreError::SecretsUnavailable)?;
        let stored = secrets.resolve_env(slug).await?;
        let token = secrets
            .resolve_provider_key(&provider.slug)
            .await?
            .ok_or_else(|| CoreError::AylulKeyMissing(provider.slug.clone()))?;
        let aylul_usage = Some(super::AylulUsageSnapshot {
            agent_slug: slug.into(),
            provider_slug: provider.slug.clone(),
            provider_name: provider.name.clone(),
            label: provider.label.clone(),
            preset: provider.preset.clone(),
        });
        let mut env = vec![("ANTHROPIC_BASE_URL".into(), provider.base_url)];
        for name in [
            "ANTHROPIC_MODEL",
            "ANTHROPIC_DEFAULT_OPUS_MODEL",
            "ANTHROPIC_DEFAULT_SONNET_MODEL",
            "ANTHROPIC_DEFAULT_HAIKU_MODEL",
        ] {
            env.push((name.into(), agent.default_model.clone()));
        }
        env.push(("CLAUDE_CODE_MAX_CONTEXT_TOKENS".into(), window.to_string()));
        if window >= 100_000 {
            env.push((
                "CLAUDE_CODE_AUTO_COMPACT_WINDOW".into(),
                window.min(1_000_000).to_string(),
            ));
        }
        let profile = ClaudeProfile::new(slug.into(), home.clone(), env);
        let spec = self
            .adapter
            .process_spec_with(&home, Some(&profile), &stored, Some(token.as_str()))
            .map_err(|_| CoreError::AylulKeyMissing(provider.slug))?;
        let person = self.owners.person.person().await?;
        let mut setup = Self::aylul_setup();
        if let Some(name) = person.aylul_calls_me.or(person.name) {
            let quoted = serde_json::to_string(&name)
                .map_err(|_| CoreError::ProfileInvalid("invalid person name".into()))?;
            setup.append = Some(format!("Call the person {quoted}."));
        }
        Ok(Opening {
            cwd: home,
            setup,
            spec,
            aylul_usage,
            aylul_models: Some((models, agent.default_model)),
        })
    }
}

/// Resolve the project's directory at opening, so a moved or deleted directory
/// fails instead of running in the daemon's own working directory.
fn workspace(context: &TurnContext) -> Result<PathBuf, String> {
    let Some(dir) = &context.project_directory else {
        return Err(
            "the project has no directory: it was created before projects owned one".into(),
        );
    };
    if !dir.is_dir() {
        return Err(format!(
            "the project directory is missing or no longer a directory: {}",
            dir.display()
        ));
    }
    Ok(dir.clone())
}
