use shadows_core::testing::{apply, approval_problems, edit_problems};
use shadows_core::{AcceptanceItem, Link, LinkKind, PlanContent, PlanOp, Problem, TaskContent};

fn task(n: u32, title: &str) -> TaskContent {
    TaskContent {
        number: n,
        title: title.into(),
        goal: format!("goal {n}"),
        reads: vec![],
        writes: vec![format!("src/t{n}")],
        acceptance: vec![AcceptanceItem {
            number: 1,
            text: format!("T{n} works"),
        }],
    }
}
fn empty() -> PlanContent {
    PlanContent {
        bindings: Vec::new(),
        title: "Login".into(),
        goal: "people can log in".into(),
        tasks: Default::default(),
        links: vec![],
    }
}
fn needs(task: u32, after: u32) -> Link {
    Link {
        task,
        after: after.into(),
        kind: LinkKind::Needs,
        label: "api".into(),
        waiting_items: vec![],
    }
}
fn messages(p: Vec<Problem>) -> Vec<String> {
    p.into_iter().map(|p| p.message).collect()
}

#[test]
fn adding_and_linking_in_one_edit_is_valid() {
    let a = apply(
        &empty(),
        &[
            PlanOp::TaskAdd {
                task: task(1, "table"),
            },
            PlanOp::TaskAdd {
                task: task(2, "api"),
            },
            PlanOp::LinkPut { link: needs(2, 1) },
        ],
    )
    .unwrap();
    assert_eq!(a.content.tasks.len(), 2);
    assert_eq!(a.changed_tasks, vec![1, 2]);
    assert_eq!(a.summary, "3 changes: added T1, added T2, linked T2 → T1");
}

#[test]
fn task_add_refuses_a_number_in_use() {
    let base = apply(
        &empty(),
        &[PlanOp::TaskAdd {
            task: task(4, "screen"),
        }],
    )
    .unwrap()
    .content;
    let err = apply(
        &base,
        &[PlanOp::TaskAdd {
            task: task(4, "other"),
        }],
    )
    .unwrap_err();
    assert_eq!(
        messages(err),
        vec!["T4 already exists; use task_update to change it"]
    );
    assert_eq!(base.tasks[&4].title, "screen");
}

#[test]
fn task_update_refuses_an_unknown_number() {
    let err = apply(&empty(), &[PlanOp::TaskUpdate { task: task(9, "x") }]).unwrap_err();
    assert_eq!(messages(err), vec!["T9 does not exist in this plan"]);
}

#[test]
fn two_operations_on_one_task_in_one_edit_are_refused() {
    let err = apply(
        &empty(),
        &[
            PlanOp::TaskAdd { task: task(1, "a") },
            PlanOp::TaskUpdate { task: task(1, "b") },
        ],
    )
    .unwrap_err();
    assert_eq!(messages(err), vec!["two operations on T1 in one edit"]);
}

#[test]
fn removing_a_linked_task_is_refused_until_its_links_go() {
    let base = apply(
        &empty(),
        &[
            PlanOp::TaskAdd { task: task(1, "a") },
            PlanOp::TaskAdd { task: task(2, "b") },
            PlanOp::LinkPut { link: needs(2, 1) },
        ],
    )
    .unwrap()
    .content;
    let err = apply(&base, &[PlanOp::TaskRemove { number: 1 }]).unwrap_err();
    assert_eq!(messages(err), vec!["T1 is still linked from T2"]);
    let ok = apply(
        &base,
        &[
            PlanOp::LinkRemove {
                task: 2,
                after: 1.into(),
                kind: LinkKind::Needs,
            },
            PlanOp::TaskRemove { number: 1 },
        ],
    );
    assert!(ok.is_ok());
}

#[test]
fn a_cycle_across_both_kinds_is_refused() {
    // T2 needs T4, and part of T4 waits for T2:
    // complete(T4) → start(T2) → complete(T2) → complete(T4).
    let base = apply(
        &empty(),
        &[
            PlanOp::TaskAdd { task: task(2, "a") },
            PlanOp::TaskAdd { task: task(4, "b") },
        ],
    )
    .unwrap()
    .content;
    let err = apply(
        &base,
        &[
            PlanOp::LinkPut { link: needs(2, 4) },
            PlanOp::LinkPut {
                link: Link {
                    task: 4,
                    after: 2.into(),
                    kind: LinkKind::CompletesAfter,
                    label: "mail".into(),
                    waiting_items: vec![1],
                },
            },
        ],
    )
    .unwrap_err();
    assert_eq!(messages(err), vec!["these links form a cycle: T2, T4"]);
}

#[test]
fn completes_after_alone_in_both_directions_is_a_cycle() {
    let base = apply(
        &empty(),
        &[
            PlanOp::TaskAdd { task: task(1, "a") },
            PlanOp::TaskAdd { task: task(2, "b") },
        ],
    )
    .unwrap()
    .content;
    let wait = |t, a: u32| Link {
        task: t,
        after: a.into(),
        kind: LinkKind::CompletesAfter,
        label: "x".into(),
        waiting_items: vec![1],
    };
    let err = apply(
        &base,
        &[
            PlanOp::LinkPut { link: wait(1, 2) },
            PlanOp::LinkPut { link: wait(2, 1) },
        ],
    )
    .unwrap_err();
    assert_eq!(messages(err), vec!["these links form a cycle: T1, T2"]);
}

#[test]
fn a_task_after_a_cycle_is_not_named_in_it() {
    let base = apply(
        &empty(),
        &[
            PlanOp::TaskAdd { task: task(1, "a") },
            PlanOp::TaskAdd { task: task(2, "b") },
            PlanOp::TaskAdd { task: task(3, "c") },
        ],
    )
    .unwrap()
    .content;
    let err = apply(
        &base,
        &[
            PlanOp::LinkPut { link: needs(1, 2) },
            PlanOp::LinkPut { link: needs(2, 1) },
            PlanOp::LinkPut { link: needs(3, 2) },
        ],
    )
    .unwrap_err();
    assert_eq!(messages(err), vec!["these links form a cycle: T1, T2"]);
}

#[test]
fn completes_after_one_way_with_needs_the_other_way_is_valid() {
    // T3 needs T2; part of T3 waits for T8 — no cycle.
    let base = apply(
        &empty(),
        &[
            PlanOp::TaskAdd {
                task: task(2, "api"),
            },
            PlanOp::TaskAdd {
                task: task(3, "reset"),
            },
            PlanOp::TaskAdd {
                task: task(8, "mail"),
            },
        ],
    )
    .unwrap()
    .content;
    let ok = apply(
        &base,
        &[
            PlanOp::LinkPut { link: needs(3, 2) },
            PlanOp::LinkPut {
                link: Link {
                    task: 3,
                    after: 8.into(),
                    kind: LinkKind::CompletesAfter,
                    label: "email sender".into(),
                    waiting_items: vec![1],
                },
            },
        ],
    );
    assert!(ok.is_ok());
}

#[test]
fn waiting_items_must_exist_on_the_waiting_task() {
    let base = apply(
        &empty(),
        &[
            PlanOp::TaskAdd { task: task(1, "a") },
            PlanOp::TaskAdd { task: task(2, "b") },
        ],
    )
    .unwrap()
    .content;
    let err = apply(
        &base,
        &[PlanOp::LinkPut {
            link: Link {
                task: 2,
                after: 1.into(),
                kind: LinkKind::CompletesAfter,
                label: "x".into(),
                waiting_items: vec![5],
            },
        }],
    )
    .unwrap_err();
    assert_eq!(messages(err), vec!["T2 has no acceptance item 5"]);
}

#[test]
fn a_link_to_a_missing_task_or_to_itself_is_refused() {
    let base = apply(&empty(), &[PlanOp::TaskAdd { task: task(1, "a") }])
        .unwrap()
        .content;
    assert_eq!(
        messages(apply(&base, &[PlanOp::LinkPut { link: needs(1, 7) }]).unwrap_err()),
        vec!["the link from T1 names T7, which does not exist"]
    );
    assert_eq!(
        messages(apply(&base, &[PlanOp::LinkPut { link: needs(1, 1) }]).unwrap_err()),
        vec!["T1 cannot be linked to itself"]
    );
}

#[test]
fn a_draft_may_lack_goals_but_approval_lists_what_is_missing() {
    let mut t = task(3, "bare");
    t.goal = String::new();
    t.acceptance.clear();
    let a = apply(&empty(), &[PlanOp::TaskAdd { task: t }]).unwrap();
    assert!(edit_problems(&a.content).is_empty());
    assert_eq!(
        messages(approval_problems(&a.content)),
        vec!["T3 has no goal", "T3 has no acceptance item"]
    );
}

#[test]
fn arabic_text_survives_an_edit() {
    let op: PlanOp = serde_json::from_str(
        r#"{"op":"task_add","task":{"number":1,
            "title":"جدول المستخدمين","goal":"تسجيل الدخول",
            "reads":[],"writes":["db/migrations"],
            "acceptance":[{"number":1,
                "text":"يظهر خطأ عند كلمة سر خاطئة"}]}}"#,
    )
    .unwrap();
    let a = apply(&empty(), &[op]).unwrap();
    assert_eq!(a.content.tasks[&1].title, "جدول المستخدمين");
    assert_eq!(
        a.content.tasks[&1].acceptance[0].text,
        "يظهر خطأ عند كلمة سر خاطئة"
    );
}

#[test]
fn removing_the_waiting_task_names_the_task_it_waits_for_once() {
    let base = apply(
        &empty(),
        &[
            PlanOp::TaskAdd { task: task(1, "a") },
            PlanOp::TaskAdd { task: task(2, "b") },
            PlanOp::LinkPut { link: needs(2, 1) },
        ],
    )
    .unwrap()
    .content;
    let err = apply(&base, &[PlanOp::TaskRemove { number: 2 }]).unwrap_err();
    assert_eq!(messages(err), vec!["T2 is still linked to T1"]);
}

#[test]
fn one_change_is_counted_in_the_singular() {
    let a = apply(&empty(), &[PlanOp::TaskAdd { task: task(1, "a") }]).unwrap();
    assert_eq!(a.summary, "1 change: added T1");
}

#[test]
fn an_empty_edit_is_refused() {
    assert_eq!(
        messages(apply(&empty(), &[]).unwrap_err()),
        vec!["an edit needs at least one operation"]
    );
}

#[test]
fn a_stored_task_number_must_be_positive() {
    let mut content = empty();
    content.tasks.insert(0, task(0, "invalid"));
    assert_eq!(
        messages(edit_problems(&content)),
        vec!["T0 is not a task number; numbers start at 1"]
    );
    assert!(
        apply(
            &content,
            &[PlanOp::PlanPut {
                title: "New title".into(),
                goal: "New goal".into(),
            }]
        )
        .is_err()
    );
}

#[test]
fn two_stored_links_of_one_kind_are_refused() {
    let mut content = empty();
    content.tasks.insert(1, task(1, "a"));
    content.tasks.insert(2, task(2, "b"));
    content.links.push(needs(2, 1));
    content.links.push(needs(2, 1));
    assert_eq!(
        messages(edit_problems(&content)),
        vec!["the needs link T2 → T1 occurs more than once"]
    );
    assert!(
        apply(
            &content,
            &[PlanOp::PlanPut {
                title: "New title".into(),
                goal: "New goal".into(),
            }]
        )
        .is_err()
    );
}
