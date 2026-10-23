//! One job: what a Planner turn sends the model — the person's text and the
//! context blocks that follow it.
//!
//! The adapter runs a slash command only when it is the whole prompt; beside
//! another block it reaches the model as plain text. So a text starting with
//! `/` goes alone when it has no images, and the changes it skipped are handed back to be withheld
//! until a normal turn can carry them.

use crate::plans::{Focus, PlanListing};
use shadows_agent::acp::{ImageBlock, PromptBlock};

/// What the harness and the turn's request supply; none of it is ordered yet.
pub(super) struct PromptParts {
    pub images: Vec<shadows_agent::acp::ImageBlock>,
    pub text: String,
    /// The harness's facts: what changed since the session last heard, and the stage line.
    pub changes: Option<String>,
    pub stage: String,
    pub focus: Option<String>,
    pub continue_plan: Option<String>,
}

/// The blocks of one prompt, in the order they are sent.
pub(super) struct Prompt {
    pub blocks: Vec<PromptBlock>,
    /// A command went without the changes, so the session has still to hear them.
    pub withholds: bool,
}

impl Prompt {
    /// Orders the person's text, images, changes since the session last heard,
    /// focus, the selected plan, then the stage line.
    pub(super) fn compose(parts: PromptParts) -> Prompt {
        let PromptParts {
            text,
            images,
            changes,
            stage,
            focus,
            continue_plan,
        } = parts;
        if images.is_empty() && Self::is_command(&text) {
            return Prompt {
                blocks: vec![PromptBlock::Text(text.trim_start().to_string())],
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
            blocks: Self::blocks(text, images, context),
            withholds: false,
        }
    }

    fn blocks(text: String, images: Vec<ImageBlock>, context: Vec<String>) -> Vec<PromptBlock> {
        std::iter::once(PromptBlock::Text(text))
            .chain(images.into_iter().map(PromptBlock::Image))
            .chain(context.into_iter().map(PromptBlock::Text))
            .collect()
    }

    pub(super) fn steered_blocks(
        text: &str,
        stage: &str,
        images: Vec<ImageBlock>,
    ) -> Vec<PromptBlock> {
        let text = if images.is_empty() || stage.is_empty() {
            Self::steered(text, stage)
        } else {
            format!("{text}\n\n{stage}")
        };
        Self::blocks(text, images, Vec::new())
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
    use super::{Prompt, PromptBlock, PromptParts};

    fn texts(values: &[&str]) -> Vec<PromptBlock> {
        values
            .iter()
            .map(|s| PromptBlock::Text((*s).into()))
            .collect()
    }

    fn parts(text: &str) -> PromptParts {
        PromptParts {
            images: Vec::new(),
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
        assert_eq!(prompt.blocks, texts(&["/context"]));
        assert!(prompt.withholds);
        assert_eq!(Prompt::steered("/context", "stage"), "/context");

        for typed in [" /context", "\n/context", "  \n/context"] {
            let indented = Prompt::compose(parts(typed));
            assert_eq!(indented.blocks, texts(&["/context"]), "{typed:?}");
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
        assert_eq!(
            prompt.blocks,
            texts(&["hello", "changed", "focus", "plan", "stage"])
        );
        assert!(!prompt.withholds);
        assert_eq!(Prompt::steered("hello", "stage"), "hello\n\nstage");

        assert_eq!(
            Prompt::compose(parts("  hello")).blocks[0],
            PromptBlock::Text("  hello".into())
        );
        assert_eq!(Prompt::steered("  hello", ""), "  hello");

        let path = Prompt::compose(parts("read src/main.rs and a/b"));
        assert_eq!(
            path.blocks,
            texts(&[
                "read src/main.rs and a/b",
                "changed",
                "focus",
                "plan",
                "stage"
            ])
        );
        assert!(!path.withholds);

        let empty = Prompt::compose(parts(""));
        assert_eq!(
            empty.blocks,
            texts(&["", "changed", "focus", "plan", "stage"])
        );
        assert_eq!(Prompt::steered("", ""), "");
    }
}
