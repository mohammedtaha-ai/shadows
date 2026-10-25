use core_turns::{TestPrompt as Prompt, TestPromptParts as PromptParts};
use shadows_agent::acp::PromptBlock;

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
