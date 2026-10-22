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
    /// A command went without the changes, so the session has still to hear them.
    pub withholds: bool,
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
        if Self::is_command(&text) {
            return Prompt {
                text: text.trim_start().to_string(),
                context: Vec::new(),
                withholds: changes.is_some(),
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
            withholds: false,
        }
    }

    /// A slash command, even after leading whitespace: a `/` further in is text.
    /// The adapter runs it only with nothing before the `/`, so a command is
    /// sent with that whitespace removed.
    fn is_command(text: &str) -> bool {
        text.trim_start().starts_with('/')
    }

    /// A steered message is one text: the stage line follows it, unless it is a command.
    pub(super) fn steered(text: &str, stage: &str) -> String {
        if Self::is_command(text) {
            text.trim_start().to_string()
        } else if stage.is_empty() {
            text.to_string()
        } else {
            format!("{text}\n\n{stage}")
        }
    }

    /// The context block that tells the Planner which task the person points
    /// at: task `number`, titled `title`, as the turn command read it.
    pub(super) fn focus_block(focus: &Focus, number: u32, title: &str) -> String {
        format!(
            "[Shadows] The person is pointing at task T{number} (\"{title}\") of plan {}, \
                revision {}. \
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
        assert!(prompt.withholds);
        assert_eq!(Prompt::steered("/context", "stage"), "/context");

        for typed in [" /context", "\n/context", "  \n/context"] {
            let indented = Prompt::compose(parts(typed));
            assert_eq!(indented.text, "/context", "{typed:?}");
            assert!(indented.context.is_empty(), "{typed:?}");
            assert!(indented.withholds, "{typed:?}");
            assert_eq!(Prompt::steered(typed, "stage"), "/context", "{typed:?}");
        }

        let unchanged = Prompt::compose(PromptParts {
            changes: None,
            ..parts("/context")
        });
        assert!(!unchanged.withholds, "nothing to withhold");
    }

    #[test]
    fn a_normal_turn_orders_its_blocks_and_withholds_nothing() {
        let prompt = Prompt::compose(parts("hello"));
        assert_eq!(prompt.context, ["changed", "focus", "plan", "stage"]);
        assert!(!prompt.withholds);
        assert_eq!(Prompt::steered("hello", "stage"), "hello\n\nstage");

        assert_eq!(Prompt::compose(parts("  hello")).text, "  hello");
        assert_eq!(Prompt::steered("  hello", ""), "  hello");

        let path = Prompt::compose(parts("read src/main.rs and a/b"));
        assert_eq!(path.context, ["changed", "focus", "plan", "stage"]);
        assert!(!path.withholds);

        let empty = Prompt::compose(parts(""));
        assert_eq!(empty.text, "");
        assert_eq!(empty.context, ["changed", "focus", "plan", "stage"]);
        assert_eq!(Prompt::steered("", ""), "");
    }
}
