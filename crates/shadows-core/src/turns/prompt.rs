//! One job: what a Planner turn sends the model — the person's text and the
//! context blocks that follow it.
//!
//! The adapter runs a slash command only when it is the whole prompt; beside
//! another block it reaches the model as plain text. So a text starting with
//! `/` goes alone, and the changes it skipped are handed back to be withheld
//! until a normal turn can carry them.

use crate::plans::{Focus, PlanListing};

/// What the harness and the turn's request supply; none of it is ordered yet.
pub(super) struct PromptParts {
    pub text: String,
    /// The harness's facts: what changed since the session last heard, and the stage line.
    pub changes: Option<String>,
    pub stage: String,
    pub focus: Option<String>,
    pub continue_plan: Option<String>,
}

/// The blocks of one prompt, in the order they are sent.
pub(super) struct Prompt {
    pub text: String,
    pub context: Vec<String>,
    /// Changes the session still has to hear: set only when a command skipped them.
    pub withheld: Option<String>,
}

impl Prompt {
    /// The one place a turn's blocks are ordered: the person's text, the
    /// changes since the session last heard, focus, the selected plan, then the
    /// stage line.
    pub(super) fn compose(parts: PromptParts) -> Prompt {
        let PromptParts {
            text,
            changes,
            stage,
            focus,
            continue_plan,
        } = parts;
        if text.starts_with('/') {
            return Prompt {
                text,
                context: Vec::new(),
                withheld: changes,
            };
        }
        let context = changes
            .into_iter()
            .chain(focus)
            .chain(continue_plan)
            .chain(std::iter::once(stage))
            .filter(|part| !part.is_empty())
            .collect();
        Prompt {
            text,
            context,
            withheld: None,
        }
    }

    /// A steered message is one text: the stage line follows it, unless it is a command.
    pub(super) fn steered(text: &str, stage: &str) -> String {
        if text.starts_with('/') || stage.is_empty() {
            text.to_string()
        } else {
            format!("{text}\n\n{stage}")
        }
    }

    /// The context block that tells the Planner which task the person points at
    /// : task `number`, titled `title`, as the turn command read it.
    pub(super) fn focus_block(focus: &Focus, number: u32, title: &str) -> String {
        format!(
            "[Shadows] The person is pointing at task T{number} (\"{title}\") of plan {}, revision {}. \
             Read the plan with workflow_get before changing it.",
            focus.workflow_id, focus.revision
        )
    }

    /// The context block for a person continuing a plan in a new conversation.
    pub(super) fn continue_plan_block(plan: &PlanListing) -> String {
        format!(
            "[Shadows] The person opened this conversation to continue the plan \"{}\" \
             (plan_id {}, latest version workflow_id {}). \
             Read it with workflow_get before you plan.",
            plan.title, plan.plan_id, plan.id
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{Prompt, PromptParts};

    fn parts(text: &str) -> PromptParts {
        PromptParts {
            text: text.into(),
            changes: Some("changed".into()),
            stage: "stage".into(),
            focus: Some("focus".into()),
            continue_plan: Some("plan".into()),
        }
    }

    #[test]
    fn a_slash_command_is_sent_as_one_block() {
        let prompt = Prompt::compose(parts("/context"));
        assert_eq!(prompt.text, "/context");
        assert!(prompt.context.is_empty());
        assert_eq!(prompt.withheld.as_deref(), Some("changed"));
        assert_eq!(Prompt::steered("/context", "stage"), "/context");
    }

    #[test]
    fn a_normal_turn_orders_its_blocks_and_withholds_nothing() {
        let prompt = Prompt::compose(parts("hello"));
        assert_eq!(prompt.context, ["changed", "focus", "plan", "stage"]);
        assert!(prompt.withheld.is_none());
        assert_eq!(Prompt::steered("hello", "stage"), "hello\n\nstage");
    }
}
