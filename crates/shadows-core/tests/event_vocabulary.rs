use shadows_core::{Place, ThreadId, UiSignal, WorkflowId};

#[test]
fn plan_show_signal_keeps_field_order_nulls_and_place_spelling() {
    for (place, spelling) in [
        (Place::Inline, "inline"),
        (Place::Side, "side"),
        (Place::Page, "page"),
    ] {
        let signal = UiSignal {
            thread_id: ThreadId::from_literal("thread-fixture"),
            target_tab: None,
            workflow_id: WorkflowId::from_literal("workflow-fixture"),
            version: 3,
            task_number: Some(7),
            place,
        };
        let expected = format!(
            concat!(
                "{{\"thread_id\":\"thread-fixture\",\"target_tab\":null,",
                "\"workflow_id\":\"workflow-fixture\",\"version\":3,",
                "\"task_number\":7,\"place\":\"{spelling}\"}}"
            ),
            spelling = spelling
        );
        assert_eq!(serde_json::to_string(&signal).unwrap(), expected);
    }
}
