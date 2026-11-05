use core_db::{CommandContext, Storage};
use core_design::{Design, DesignAnchor};
use core_grants::Grants;
use core_operations::Operations;
use core_plans::Plans;
use core_projects::{ProjectId, Projects};
use core_threads::{ThreadId, Threads};
use core_turns::TestPlanning;
use std::sync::Arc;

struct Fixture {
    _dir: tempfile::TempDir,
    storage: Arc<Storage>,
    design: Design,
    plans: Plans,
    thread: ThreadId,
    project: ProjectId,
}

impl Fixture {
    async fn edit(&self, ops: Vec<core_design::WorkspaceOp>) {
        let revision = self
            .design
            .parts(&self.project, None, None)
            .await
            .unwrap()
            .revision;
        let project = self.project.clone();
        self.storage
            .write_txn_as::<_, _, core_design::DesignError>(move |conn| {
                Box::pin(async move {
                    Design::edit_workspace_in(
                        conn,
                        &CommandContext::user(
                            uuid::Uuid::new_v4().to_string(),
                            "design.edit",
                            serde_json::json!({}),
                        ),
                        &project,
                        revision,
                        ops,
                        &Default::default(),
                    )
                    .await?;
                    Ok(())
                })
            })
            .await
            .unwrap();
    }
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let storage = Arc::new(Storage::open(&dir.path().join("test.db")).await.unwrap());
        let project = Projects::new(storage.clone())
            .create(
                "project".into(),
                "demo",
                "Demo",
                dir.path().to_str().unwrap(),
            )
            .await
            .unwrap();
        let thread = Threads::create_planning_thread_for_test(
            &storage,
            &CommandContext::user("thread".into(), "thread.create", serde_json::json!({})),
            &project.id,
            "Planner",
            "claude-code",
            None,
        )
        .await
        .unwrap();
        let operations = Operations::new(storage.clone());
        let design = Design::new(storage.clone(), operations.clone());
        design.seed_roadmap_for_tests(&project.id).await.unwrap();
        let plans = Plans::new(
            storage.clone(),
            Grants::new(storage.clone(), String::new()),
            operations,
            design.clone(),
        );
        Self {
            _dir: dir,
            storage,
            design,
            plans,
            thread: thread.id,
            project: project.id,
        }
    }

    async fn compile(&self) -> core_harness::CarriedContext {
        TestPlanning::compile(
            &self.design,
            &self.plans,
            &self.thread,
            &self.project,
            1_500,
            8_000,
        )
        .await
        .unwrap()
    }

    async fn anchor(&self) {
        let part = self
            .design
            .parts(&self.project, None, None)
            .await
            .unwrap()
            .items
            .remove(0);
        self.design
            .set_thread_anchor(
                "anchor".into(),
                &self.thread,
                Some(DesignAnchor::Part(part.id)),
            )
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn the_index_carries_purpose_not_the_whole_vision() {
    use core_design::{VisionField, WorkspaceOp};
    let f = Fixture::new().await;
    f.edit(vec![
        WorkspaceOp::VisionItemAdd {
            field: VisionField::Purpose,
            text: "A useful tool".into(),
        },
        WorkspaceOp::VisionItemAdd {
            field: VisionField::TechnicalDirection,
            text: "Hidden technical direction".into(),
        },
    ])
    .await;
    let context = f.compile().await;
    assert!(context.text.contains("A useful tool"));
    assert!(!context.text.contains("Hidden technical direction"));
    f.storage.close().await.unwrap();
}

#[tokio::test]
async fn parts_past_one_page_are_announced_not_listed() {
    use core_design::{PartContent, PartId, WorkspaceOp};
    let f = Fixture::new().await;
    f.edit(
        (0..51)
            .map(|n| WorkspaceOp::PartCreate {
                id: PartId::generate(),
                parent: None,
                before: None,
                content: PartContent {
                    title: format!("Part {n}"),
                    folders: vec![format!("part-{n}")],
                    ..Default::default()
                },
            })
            .collect(),
    )
    .await;
    assert!(f.compile().await.text.contains("[more parts available]"));
    f.storage.close().await.unwrap();
}

#[tokio::test]
async fn an_outcome_anchor_lists_its_parts_and_plans() {
    use core_design::{OutcomeContent, OutcomeId, WorkspaceOp};
    let f = Fixture::new().await;
    let part = f
        .design
        .parts(&f.project, None, None)
        .await
        .unwrap()
        .items
        .remove(0)
        .id;
    let id = OutcomeId::generate();
    f.edit(vec![
        WorkspaceOp::OutcomeCreate {
            id: id.clone(),
            parent: None,
            before: None,
            content: OutcomeContent {
                title: "Stage one".into(),
                intended_result: "Working software".into(),
                acceptance: vec![],
            },
        },
        WorkspaceOp::OutcomePartPut {
            outcome: id.clone(),
            part,
        },
    ])
    .await;
    f.design
        .set_thread_anchor("anchor".into(), &f.thread, Some(DesignAnchor::Outcome(id)))
        .await
        .unwrap();
    let context = f.compile().await;
    assert!(context.text.contains("Working software"));
    assert!(context.text.contains("Seeded part"));
    f.storage.close().await.unwrap();
}

#[tokio::test]
async fn an_unanchored_thread_gets_the_index_only() {
    let f = Fixture::new().await;
    let context = f.compile().await;
    assert!(context.text.contains("Seeded part"));
    assert_eq!(context.detail_tokens, 0);
    f.storage.close().await.unwrap();
}

#[tokio::test]
async fn a_part_anchor_adds_its_detail_and_sources() {
    let f = Fixture::new().await;
    f.anchor().await;
    let context = f.compile().await;
    assert!(context.text.contains("Seeded for tests"));
    assert!(context.detail_tokens > 0);
    assert!(
        context
            .sources
            .iter()
            .any(|source| source.kind == "part" && source.version == 1)
    );
    f.storage.close().await.unwrap();
}

#[tokio::test]
async fn the_same_design_gives_the_same_digest() {
    let f = Fixture::new().await;
    f.anchor().await;
    assert_eq!(f.compile().await, f.compile().await);
    f.storage.close().await.unwrap();
}

#[tokio::test]
async fn the_index_keeps_each_childs_parent() {
    use core_design::{PartContent, PartId, WorkspaceOp};
    let f = Fixture::new().await;
    let first = f
        .design
        .parts(&f.project, None, None)
        .await
        .unwrap()
        .items
        .remove(0)
        .id;
    let second = PartId::generate();
    f.edit(vec![
        WorkspaceOp::PartCreate {
            id: second.clone(),
            parent: None,
            before: None,
            content: PartContent {
                title: "Other root".into(),
                folders: vec!["other-root".into()],
                ..Default::default()
            },
        },
        WorkspaceOp::PartCreate {
            id: PartId::generate(),
            parent: Some(first.clone()),
            before: None,
            content: PartContent {
                title: "First child".into(),
                folders: vec!["seeded/first-child".into()],
                ..Default::default()
            },
        },
        WorkspaceOp::PartCreate {
            id: PartId::generate(),
            parent: Some(second.clone()),
            before: None,
            content: PartContent {
                title: "Second child".into(),
                folders: vec!["other-root/second-child".into()],
                ..Default::default()
            },
        },
    ])
    .await;
    let context = f.compile().await;
    let first_child = context
        .text
        .lines()
        .find(|line| line.contains("First child"))
        .unwrap();
    let second_child = context
        .text
        .lines()
        .find(|line| line.contains("Second child"))
        .unwrap();
    assert!(first_child.contains(first.as_str()));
    assert!(!first_child.contains(second.as_str()));
    assert!(second_child.contains(second.as_str()));
    assert!(!second_child.contains(first.as_str()));
    f.storage.close().await.unwrap();
}

#[tokio::test]
async fn a_detail_still_over_after_every_cut_is_sent_and_says_by_how_much() {
    let f = Fixture::new().await;
    f.anchor().await;
    let context = TestPlanning::compile(&f.design, &f.plans, &f.thread, &f.project, 1_500, 1)
        .await
        .unwrap();
    assert!(context.text.contains("Seeded for tests"));
    assert!(context.over_tokens > 0);
    f.storage.close().await.unwrap();
}

#[tokio::test]
async fn a_detail_cut_keeps_child_identities_for_both_anchor_kinds() {
    use core_design::{OutcomeContent, OutcomeId, PartContent, PartId, WorkspaceOp};
    for outcome_anchor in [false, true] {
        let f = Fixture::new().await;
        let part = f
            .design
            .parts(&f.project, None, None)
            .await
            .unwrap()
            .items
            .remove(0)
            .id;
        let child_part = PartId::generate();
        let outcome = OutcomeId::generate();
        let child_outcome = OutcomeId::generate();
        f.edit(vec![
            WorkspaceOp::PartCreate {
                id: child_part.clone(),
                parent: Some(part.clone()),
                before: None,
                content: PartContent {
                    title: "Part child identity".into(),
                    folders: vec!["seeded/child-identity".into()],
                    responsibility: "Optional part description".repeat(100),
                    ..Default::default()
                },
            },
            WorkspaceOp::OutcomeCreate {
                id: outcome.clone(),
                parent: None,
                before: None,
                content: OutcomeContent {
                    title: "Outcome anchor".into(),
                    intended_result: "Root result".into(),
                    acceptance: vec![],
                },
            },
            WorkspaceOp::OutcomeCreate {
                id: child_outcome.clone(),
                parent: Some(outcome.clone()),
                before: None,
                content: OutcomeContent {
                    title: "Outcome child identity".into(),
                    intended_result: "Optional outcome description".repeat(100),
                    acceptance: vec![],
                },
            },
        ])
        .await;
        let (anchor, id, title, description) = if outcome_anchor {
            (
                DesignAnchor::Outcome(outcome),
                child_outcome.to_string(),
                "Outcome child identity",
                "Optional outcome description",
            )
        } else {
            (
                DesignAnchor::Part(part),
                child_part.to_string(),
                "Part child identity",
                "Optional part description",
            )
        };
        f.design
            .set_thread_anchor("anchor".into(), &f.thread, Some(anchor))
            .await
            .unwrap();
        let context = TestPlanning::compile(&f.design, &f.plans, &f.thread, &f.project, 1, 1)
            .await
            .unwrap();
        let detail = context
            .text
            .split("## Planning anchor detail")
            .nth(1)
            .unwrap();
        assert!(detail.contains(&format!("Child {id}: {title}")));
        assert!(!context.text.contains(description));
        assert!(context.sources.iter().any(|source| source.id == id));
        f.storage.close().await.unwrap();
    }
}

#[tokio::test]
async fn sources_follow_surviving_content_and_keep_shared_sources() {
    use core_design::{PartContent, PartId, WorkspaceOp};
    let f = Fixture::new().await;
    let root = f
        .design
        .parts(&f.project, None, None)
        .await
        .unwrap()
        .items
        .remove(0)
        .id;
    let child = PartId::generate();
    let content = PartContent {
        title: "Omitted child".into(),
        folders: vec!["seeded/omitted".into()],
        ..Default::default()
    };
    f.edit(vec![WorkspaceOp::PartCreate {
        id: child.clone(),
        parent: Some(root),
        before: None,
        content: content.clone(),
    }])
    .await;
    let compile = || TestPlanning::compile(&f.design, &f.plans, &f.thread, &f.project, 1, 8_000);
    let before = compile().await.unwrap();
    assert!(
        !before
            .sources
            .iter()
            .any(|source| source.id == child.as_str())
    );
    f.edit(vec![WorkspaceOp::PartPut {
        id: child.clone(),
        content: PartContent {
            responsibility: "Revised omitted content".into(),
            ..content
        },
    }])
    .await;
    let revised = compile().await.unwrap();
    assert!(
        !revised
            .sources
            .iter()
            .any(|source| source.id == child.as_str())
    );
    assert_eq!(
        before
            .sources
            .iter()
            .filter(|source| source.kind != "vision")
            .collect::<Vec<_>>(),
        revised
            .sources
            .iter()
            .filter(|source| source.kind != "vision")
            .collect::<Vec<_>>(),
    );
    f.design
        .set_thread_anchor(
            "anchor".into(),
            &f.thread,
            Some(DesignAnchor::Part(child.clone())),
        )
        .await
        .unwrap();
    let anchored = compile().await.unwrap();
    assert!(
        anchored
            .sources
            .iter()
            .any(|source| source.id == child.as_str())
    );
    f.storage.close().await.unwrap();
}
