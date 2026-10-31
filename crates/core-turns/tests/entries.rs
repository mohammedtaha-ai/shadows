use core_turns::{TestCollector as Collector, TestDurable as Durable};
use shadows_agent::events::HarnessEvent;
use shadows_agent::events::{AgentFacts, SubagentCard};

#[test]
fn the_last_plan_update_becomes_one_todo_list_entry_at_the_end() {
    use shadows_agent::events::{TodoItem, TodoStatus};
    let mut collector = Collector::new(None);
    let first = TodoItem {
        content: "Finish".into(),
        status: TodoStatus::Pending,
    };
    let last = TodoItem {
        content: "Finish".into(),
        status: TodoStatus::Completed,
    };
    collector.push(&HarnessEvent::Todos(vec![first]));
    collector.push(&chunk("m", "done"));
    collector.push(&HarnessEvent::Todos(vec![last.clone()]));
    assert_eq!(collector.live().1, Some(vec![last.clone()]));
    let result = collector.finish();
    assert_eq!(
        result,
        [
            Durable::Message("done".into()),
            Durable::TodoList(vec![last])
        ]
    );
}

#[test]
fn a_turn_without_a_plan_writes_no_todo_list() {
    let mut collector = Collector::new(None);
    collector.push(&chunk("m", "done"));
    assert_eq!(collector.finish(), [Durable::Message("done".into())]);
    collector.push(&HarnessEvent::Todos(vec![]));
    assert!(collector.finish().is_empty());
}

#[test]
fn changed_tools_reports_a_call_once_per_change() {
    use shadows_agent::events::CallStatus;
    let mut collector = Collector::new(None);
    collector.push(&tool("a", Some("Read"), Some("pending")));
    assert_eq!(collector.changed_tools()[0].status, CallStatus::Pending);
    assert_eq!(collector.live().0[0].id, "a");
    assert!(collector.changed_tools().is_empty());
    collector.push(&tool("a", Some("Read"), Some("pending")));
    assert!(collector.changed_tools().is_empty());
    collector.push(&tool("a", None, Some("completed")));
    assert_eq!(collector.changed_tools()[0].status, CallStatus::Completed);
    assert!(collector.live().0.is_empty());
}
fn chunk(id: &str, text: &str) -> HarnessEvent {
    HarnessEvent::Chunk {
        message_id: Some(id.into()),
        text: text.into(),
    }
}
fn tool(id: &str, title: Option<&str>, status: Option<&str>) -> HarnessEvent {
    HarnessEvent::ToolCall {
        facts: Default::default(),
        id: id.into(),
        title: title.map(str::to_owned),
        status: status.map(str::to_owned),
        tool: None,
        parent: None,
        agent: None,
    }
}

#[test]
fn a_finished_call_waits_for_late_updates_until_the_next_message() {
    let mut collector = Collector::new(None);
    collector.push(&tool("edit", Some("Edit file"), Some("in_progress")));
    assert!(
        collector
            .push(&tool("edit", None, Some("completed")))
            .is_empty()
    );
    assert!(!collector.push(&chunk("next", "reply")).is_empty());
}
fn agent(id: &str, status: Option<&str>, facts: AgentFacts) -> HarnessEvent {
    HarnessEvent::ToolCall {
        facts: Default::default(),
        id: id.into(),
        title: Some("Task".into()),
        status: status.map(str::to_owned),
        tool: Some("Agent".into()),
        parent: None,
        agent: Some(Box::new(facts)),
    }
}
fn step(id: &str, tool: &str, title: &str, parent: Option<&str>, status: &str) -> HarnessEvent {
    HarnessEvent::ToolCall {
        facts: Default::default(),
        id: id.into(),
        title: Some(title.into()),
        status: Some(status.into()),
        tool: Some(tool.into()),
        parent: parent.map(str::to_owned),
        agent: None,
    }
}
fn cards(out: &[Durable]) -> Vec<&SubagentCard> {
    out.iter()
        .filter_map(|d| match d {
            Durable::Subagent(c) => Some(&**c),
            _ => None,
        })
        .collect()
}
#[test]
fn a_subagent_becomes_one_entry_with_its_steps_and_numbers() {
    let mut c = Collector::new(None);
    let mut out = Vec::new();
    let asked = AgentFacts {
        description: Some("List files".into()),
        model: Some("sonnet".into()),
        prompt: Some("List them".into()),
        ..AgentFacts::default()
    };
    let numbers = AgentFacts {
        resolved_model: Some("claude-sonnet-5-5".into()),
        tokens: Some(43119),
        report: Some("two files".into()),
        ..AgentFacts::default()
    };
    for e in [
        chunk("m1", "launching"),
        agent("a", Some("pending"), asked),
        step("r1", "Read", "Read alpha.txt", Some("a"), "pending"),
        step("r1", "Read", "Read alpha.txt", None, "completed"),
        step("r2", "Glob", "Find *", Some("a"), "completed"),
        step("h", "SubagentHandback", "Hand back", Some("a"), "completed"),
        agent("a", Some("completed"), AgentFacts::default()),
    ] {
        out.extend(c.push(&e));
    }
    assert!(cards(&out).is_empty(), "no numbers yet: {out:?}");
    out.extend(c.push(&agent("a", None, numbers)));
    out.extend(c.push(&chunk("m2", "done")));
    out.extend(c.finish());
    let card = cards(&out)[0].clone();
    assert_eq!(out.len(), 3, "an inner tool writes no entry: {out:?}");
    assert_eq!(out[0], Durable::Message("launching".into()));
    assert_eq!(out[2], Durable::Message("done".into()));
    assert_eq!(card.title, "List files");
    assert_eq!(card.steps, ["Read alpha.txt", "Find *"]);
    assert_eq!(card.model.as_deref(), Some("claude-sonnet-5-5"));
    assert_eq!(
        (card.status.as_str(), card.tokens),
        ("completed", Some(43119))
    );
    assert_eq!(card.report.as_deref(), Some("two files"));
    assert!(!c.changed_cards().is_empty());
}
#[test]
fn a_card_without_numbers_goes_before_the_next_text_and_a_running_one_stops() {
    let mut c = Collector::new(None);
    c.push(&agent("a", Some("pending"), AgentFacts::default()));
    c.push(&agent("a", Some("failed"), AgentFacts::default()));
    let out = c.push(&chunk("m1", "it failed"));
    assert_eq!(cards(&out)[0].status, "failed");
    // A late update to a written card opens no second one.
    let out = c.push(&agent("a", None, AgentFacts::default()));
    assert_eq!(out, [Durable::Message("it failed".into())]);
    assert!(
        c.push(&agent("b", Some("pending"), AgentFacts::default()))
            .is_empty()
    );
    let last = c.finish();
    assert_eq!(cards(&last).len(), 1, "only b: {last:?}");
    assert_eq!(cards(&last)[0].status, "stopped");
}
#[test]
fn chunks_of_one_message_become_one_entry_and_a_tool_call_splits_messages() {
    let mut c = Collector::new(None);
    let mut out = Vec::new();
    for e in [
        chunk("m1", "hello "),
        chunk("m1", "there"),
        tool("t1", Some("Terminal"), Some("pending")),
        tool("t1", Some("Read notes.md"), None),
        tool("t1", None, Some("completed")),
        chunk("m2", "done"),
    ] {
        out.extend(c.push(&e));
    }
    out.extend(c.finish());
    assert_eq!(out.len(), 3);
    assert_eq!(out[0], Durable::Message("hello there".into()));
    assert!(matches!(&out[1], Durable::Tool { title, .. } if title == "Read notes.md"));
    assert_eq!(out[2], Durable::Message("done".into()));
}
#[test]
fn a_tool_call_that_never_finished_is_emitted_at_the_end_under_its_last_title() {
    let mut c = Collector::new(None);
    assert!(
        c.push(&tool("t9", Some("Terminal"), Some("pending")))
            .is_empty()
    );
    assert!(c.push(&tool("t9", Some("npm install"), None)).is_empty());
    assert!(matches!(&c.finish()[0], Durable::Tool { title, .. } if title == "npm install"));
}
#[test]
fn unfinished_tools_are_emitted_in_the_order_they_began() {
    let mut c = Collector::new(None);
    for id in ["a", "b", "c"] {
        assert!(c.push(&tool(id, Some(id), Some("pending"))).is_empty());
    }
    let out = c.finish();
    let titles: Vec<_> = out
        .iter()
        .map(|entry| match entry {
            Durable::Tool { title, .. } => title.as_str(),
            _ => panic!("expected tool"),
        })
        .collect();
    assert_eq!(titles, ["a", "b", "c"]);
}
#[test]
fn a_cut_writes_the_streamed_text_and_the_reply_continues_as_a_new_message() {
    let mut c = Collector::new(None);
    assert!(c.push(&chunk("m1", "so far")).is_empty());
    assert_eq!(c.cut(), [Durable::Message("so far".into())]);
    assert!(c.push(&chunk("m1", " and on")).is_empty());
    assert_eq!(c.finish(), [Durable::Message(" and on".into())]);
}
#[test]
fn a_changed_message_id_closes_the_previous_message() {
    let mut c = Collector::new(None);
    assert!(c.push(&chunk("m1", "a")).is_empty());
    assert_eq!(c.push(&chunk("m2", "b")), [Durable::Message("a".into())]);
    assert_eq!(c.finish(), [Durable::Message("b".into())]);
    assert!(c.finish().is_empty());
}

fn with_facts(id: &str, status: &str, facts: shadows_agent::events::ToolFacts) -> HarnessEvent {
    HarnessEvent::ToolCall {
        id: id.into(),
        title: Some("call".into()),
        status: Some(status.into()),
        tool: Some("Edit".into()),
        parent: None,
        agent: None,
        facts,
    }
}

fn tool_cards(entries: &[Durable]) -> Vec<&shadows_agent::events::ToolCallCard> {
    entries
        .iter()
        .filter_map(|entry| match entry {
            Durable::Tool { card, .. } => Some(card.as_ref()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_tool_entry_carries_its_card_with_target_lines_and_status() {
    use shadows_agent::events::{CallKind, CallStatus, ToolFacts};
    let mut c = Collector::new(None);
    c.push(&with_facts(
        "r",
        "completed",
        ToolFacts {
            kind: Some(CallKind::Read),
            input: Some(serde_json::json!({"path":"a.rs","offset":10,"limit":5})),
            ..Default::default()
        },
    ));
    let out = c.finish();
    let card = tool_cards(&out)[0];
    assert_eq!(card.target.as_deref(), Some("a.rs"));
    assert_eq!(card.lines, Some([10, 14]));
    assert_eq!(card.status, CallStatus::Completed);
    assert_eq!(card.tool.as_deref(), Some("Edit"));
    assert!(!card.started_at.is_empty() && !card.finished_at.is_empty());
}

#[test]
fn a_placed_diff_after_the_end_replaces_the_snippet_before_the_next_text() {
    use shadows_agent::events::{CallKind, ToolDiff, ToolFacts, ToolLocation};
    let mut c = Collector::new(None);
    c.push(&with_facts(
        "e",
        "completed",
        ToolFacts {
            kind: Some(CallKind::Edit),
            diffs: Some(vec![ToolDiff {
                path: "a".into(),
                old_text: Some("x\n".into()),
                new_text: "y\n".into(),
            }]),
            ..Default::default()
        },
    ));
    c.push(&with_facts(
        "e",
        "completed",
        ToolFacts {
            locations: Some(vec![
                ToolLocation {
                    path: "a".into(),
                    line: Some(10),
                },
                ToolLocation {
                    path: "a".into(),
                    line: Some(20),
                },
            ]),
            diffs: Some(vec![
                ToolDiff {
                    path: "a".into(),
                    old_text: Some("x\n".into()),
                    new_text: "x\ny\n".into(),
                },
                ToolDiff {
                    path: "a".into(),
                    old_text: Some("q\n".into()),
                    new_text: "r\n".into(),
                },
            ]),
            ..Default::default()
        },
    ));
    let out = c.push(&chunk("m", "done"));
    let card = tool_cards(&out)[0];
    let hunks = card.diff.as_ref().unwrap();
    assert_eq!(
        (hunks[0].old_start, hunks[0].new_start),
        (Some(10), Some(10))
    );
    assert_eq!(
        (hunks[1].old_start, hunks[1].new_start),
        (Some(19), Some(20))
    );
    assert_eq!(
        (card.added, card.removed, card.lines),
        (Some(2), Some(1), Some([10, 20]))
    );
}

#[test]
fn a_diff_over_400_lines_keeps_its_counts_and_drops_its_hunks() {
    use shadows_agent::events::{ToolDiff, ToolFacts};
    let mut c = Collector::new(None);
    c.push(&with_facts(
        "e",
        "completed",
        ToolFacts {
            diffs: Some(vec![ToolDiff {
                path: "a".into(),
                old_text: None,
                new_text: (0..401).map(|n| format!("{n}\n")).collect(),
            }]),
            ..Default::default()
        },
    ));
    let out = c.finish();
    let card = tool_cards(&out)[0];
    assert_eq!((card.added, card.removed), (Some(401), Some(0)));
    assert!(card.diff.is_none());
}

#[test]
fn a_new_file_is_one_hunk_of_added_lines_without_numbers() {
    use shadows_agent::events::{DiffOp, ToolDiff, ToolFacts};
    let mut c = Collector::new(None);
    c.push(&with_facts(
        "e",
        "completed",
        ToolFacts {
            diffs: Some(vec![ToolDiff {
                path: "a".into(),
                old_text: None,
                new_text: "one\ntwo\n".into(),
            }]),
            ..Default::default()
        },
    ));
    let out = c.finish();
    let hunk = &tool_cards(&out)[0].diff.as_ref().unwrap()[0];
    assert_eq!((hunk.old_start, hunk.new_start), (None, None));
    assert_eq!(hunk.lines.len(), 2);
    assert!(hunk.lines.iter().all(|line| line.op == DiffOp::Add));
}

#[test]
fn a_failed_call_is_failed_and_an_open_one_is_stopped() {
    use shadows_agent::events::{CallStatus, ToolFacts};
    let mut c = Collector::new(None);
    c.push(&with_facts("f", "failed", ToolFacts::default()));
    c.push(&with_facts("s", "in_progress", ToolFacts::default()));
    let out = c.finish();
    let cards = tool_cards(&out);
    assert_eq!(
        cards.iter().map(|c| &c.status).collect::<Vec<_>>(),
        [&CallStatus::Failed, &CallStatus::Stopped]
    );
    assert!(cards.iter().all(|c| !c.finished_at.is_empty()));
}

#[test]
fn a_late_failure_updates_an_ended_call_before_it_is_written() {
    use shadows_agent::events::{CallStatus, ToolFacts};
    let mut c = Collector::new(None);
    c.push(&with_facts("e", "completed", ToolFacts::default()));
    c.push(&with_facts(
        "e",
        "failed",
        ToolFacts {
            texts: Some(vec!["failed later".into()]),
            ..Default::default()
        },
    ));
    let out = c.finish();
    let card = tool_cards(&out)[0];
    assert_eq!(card.status, CallStatus::Failed);
    assert_eq!(card.error.as_deref(), Some("failed later"));
}

#[test]
fn a_failed_call_keeps_its_error_text_cut_at_2000_characters() {
    use shadows_agent::events::ToolFacts;
    let mut c = Collector::new(None);
    c.push(&with_facts(
        "a",
        "failed",
        ToolFacts {
            texts: Some(vec![format!("  {}  ", "é".repeat(2001))]),
            output: Some(serde_json::json!("ignored")),
            ..Default::default()
        },
    ));
    c.push(&with_facts(
        "b",
        "failed",
        ToolFacts {
            output: Some(serde_json::json!({"code":1})),
            ..Default::default()
        },
    ));
    c.push(&with_facts(
        "c",
        "completed",
        ToolFacts {
            texts: Some(vec!["error".into()]),
            ..Default::default()
        },
    ));
    let out = c.finish();
    let cards = tool_cards(&out);
    let error = cards[0].error.as_ref().unwrap();
    assert_eq!(error.chars().count(), 2000);
    assert!(error.ends_with('…'));
    assert_eq!(cards[1].error.as_deref(), Some("{\"code\":1}"));
    assert_eq!(cards[2].error, None);
}

#[test]
fn a_target_under_the_root_is_relative() {
    use shadows_agent::events::{ToolFacts, ToolLocation};
    let root = std::env::temp_dir().join("project");
    let path = root.join("folder").join("file.rs");
    let mut c = Collector::new(Some(root));
    c.push(&with_facts(
        "e",
        "completed",
        ToolFacts {
            locations: Some(vec![ToolLocation {
                path: path.to_string_lossy().into_owned(),
                line: None,
            }]),
            ..Default::default()
        },
    ));
    let out = c.finish();
    assert_eq!(
        tool_cards(&out)[0].target.as_deref(),
        Some("folder/file.rs")
    );
}

#[test]
fn an_update_for_a_written_call_writes_nothing() {
    use shadows_agent::events::ToolFacts;
    let mut c = Collector::new(None);
    c.push(&with_facts("e", "completed", ToolFacts::default()));
    assert_eq!(tool_cards(&c.push(&chunk("m", "done"))).len(), 1);
    assert!(
        c.push(&with_facts("e", "completed", ToolFacts::default()))
            .is_empty()
    );
    assert_eq!(c.finish(), [Durable::Message("done".into())]);
}
