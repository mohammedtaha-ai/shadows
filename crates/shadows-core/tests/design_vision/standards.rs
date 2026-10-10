use super::*;

async fn core_and_project(tmp: &tempfile::TempDir) -> (Arc<AppCore>, ProjectId) {
    let storage = Arc::new(
        Storage::open(&tmp.path().join("standards.sqlite3"))
            .await
            .unwrap(),
    );
    let runtime = Arc::new(Runtime::start(storage.clone()).await.unwrap().0);
    let core = AppCore::assemble(CoreParts {
        storage: storage.clone(),
        runtime,
        sessions: acp::fake_sessions(storage.clone()),
        handles: Arc::new(LiveHandles::default()),
        bus: tokio::sync::broadcast::channel(32).0,
        ui: tokio::sync::broadcast::channel(32).0,
        mcp_url: acp::MCP_URL.into(),
    });
    let project = core
        .projects()
        .create(
            "project".into(),
            "standards",
            "Standards",
            tmp.path().to_str().unwrap(),
        )
        .await
        .unwrap()
        .id;
    (core, project)
}

fn rule(id: &str, parts: &[&str]) -> StandardRule {
    StandardRule {
        id: id.into(),
        text: "Every payment is logged.".into(),
        parts: parts.iter().map(|p| p.to_string()).collect(),
    }
}

#[tokio::test]
async fn standards_without_additions_are_the_base() {
    let tmp = tempfile::tempdir().unwrap();
    let (core, project) = core_and_project(&tmp).await;
    let standards = core.design().standards(&project).await.unwrap();
    assert_eq!(&standards.base, shadows_core::base_standards());
    assert_eq!(standards.additions, None);
    assert_eq!(
        standards.mandatory_parts(),
        ["backend", "database", "api", "frontend"]
    );
    let unknown = ProjectId::generate();
    assert!(core.design().standards(&unknown).await.is_err());
    assert!(core.design().stage(&unknown).await.is_err());
}

#[tokio::test]
async fn each_additions_save_is_a_new_version_and_a_replay_saves_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let (core, project) = core_and_project(&tmp).await;
    let content = StandardsAdditions {
        rules: vec![rule("P1", &["billing"])],
        parts: vec![AdditionalPart {
            name: "billing".into(),
            owns: "Payments.".into(),
        }],
    };
    let first = core
        .design()
        .save_standards_additions("a1".into(), &project, content.clone())
        .await
        .unwrap();
    assert_eq!(first.number, 1);
    assert!(!first.id.is_empty());
    assert!(serde_json::to_value(&first).unwrap().get("id").is_none());
    let second = core
        .design()
        .save_standards_additions("a2".into(), &project, Default::default())
        .await
        .unwrap();
    assert_eq!(second.number, 2);
    assert_eq!(
        core.design().standards(&project).await.unwrap().additions,
        Some(second)
    );
    assert_eq!(
        core.design()
            .save_standards_additions("a1".into(), &project, content.clone(),)
            .await
            .unwrap(),
        first
    );
    let mut changed = content.clone();
    changed.parts[0].owns = "Other.".into();
    assert!(matches!(
        core.design()
            .save_standards_additions("a1".into(), &project, changed,)
            .await,
        Err(CoreError::Storage(StorageError::CommandConflict))
    ));
    core.projects()
        .remove("remove".into(), &project)
        .await
        .unwrap();
    assert!(core.design().standards(&project).await.is_err());
    assert!(core.design().stage(&project).await.is_err());
    assert_eq!(
        core.design()
            .save_standards_additions("a1".into(), &project, content,)
            .await
            .unwrap(),
        first
    );
    assert!(
        core.design()
            .save_standards_additions("removed".into(), &project, Default::default(),)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn additions_cannot_repeat_a_base_part_or_name_an_unknown_one() {
    let tmp = tempfile::tempdir().unwrap();
    let (core, project) = core_and_project(&tmp).await;
    let refused = [
        StandardsAdditions {
            parts: vec![AdditionalPart {
                name: "backend".into(),
                owns: "x".into(),
            }],
            ..Default::default()
        },
        StandardsAdditions {
            rules: vec![rule("P1", &["billing"])],
            ..Default::default()
        },
        StandardsAdditions {
            rules: vec![rule("S1", &[])],
            ..Default::default()
        },
        StandardsAdditions {
            rules: vec![rule("P0", &[])],
            ..Default::default()
        },
        StandardsAdditions {
            rules: vec![rule("P01", &[])],
            ..Default::default()
        },
        StandardsAdditions {
            rules: vec![rule("P1", &[]), rule("P1", &[])],
            ..Default::default()
        },
        StandardsAdditions {
            parts: vec![AdditionalPart {
                name: "Billing".into(),
                owns: "x".into(),
            }],
            ..Default::default()
        },
        StandardsAdditions {
            parts: vec![AdditionalPart {
                name: "billing".into(),
                owns: "  ".into(),
            }],
            ..Default::default()
        },
        StandardsAdditions {
            rules: vec![StandardRule {
                text: "  ".into(),
                ..rule("P1", &[])
            }],
            ..Default::default()
        },
    ];
    for (i, content) in refused.into_iter().enumerate() {
        let error = core
            .design()
            .save_standards_additions(format!("bad-{i}"), &project, content)
            .await;
        assert!(
            matches!(
                &error,
                Err(CoreError::Refused {
                    code: shadows_core::ErrorCode::InvalidCommand,
                    ..
                })
            ),
            "case {i}: {error:?}"
        );
    }
    assert_eq!(
        core.design().standards(&project).await.unwrap().additions,
        None
    );
}

#[tokio::test]
async fn standards_saves_are_atomic_and_survive_restart() {
    let tmp = tempfile::tempdir().unwrap();
    let (core, project) = core_and_project(&tmp).await;
    let storage = Storage::open(&tmp.path().join("standards.sqlite3"))
        .await
        .unwrap();
    let (a, b) = tokio::join!(
        core.design()
            .save_standards_additions("a".into(), &project, Default::default()),
        core.design()
            .save_standards_additions("b".into(), &project, Default::default()),
    );
    let mut numbers = [a.unwrap().number, b.unwrap().number];
    numbers.sort();
    assert_eq!(numbers, [1, 2]);
    sqlx::query(
        "CREATE TRIGGER refuse_standards_event BEFORE INSERT ON durable_event
         WHEN NEW.kind = 'ProjectStandardsSaved'
         BEGIN SELECT RAISE(ABORT, 'test refusal'); END",
    )
    .execute(storage.reader())
    .await
    .unwrap();
    assert!(
        core.design()
            .save_standards_additions("rollback".into(), &project, Default::default(),)
            .await
            .is_err()
    );
    assert_eq!(
        core.design()
            .standards(&project)
            .await
            .unwrap()
            .additions
            .unwrap()
            .number,
        2
    );
    sqlx::query("DROP TRIGGER refuse_standards_event")
        .execute(storage.reader())
        .await
        .unwrap();
    assert_eq!(
        core.design()
            .save_standards_additions("rollback".into(), &project, Default::default(),)
            .await
            .unwrap()
            .number,
        3
    );
    let reopened = Storage::open(&tmp.path().join("standards.sqlite3"))
        .await
        .unwrap();
    assert_eq!(
        reopened
            .current_standards_additions(&project)
            .await
            .unwrap()
            .unwrap()
            .number,
        3
    );
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM durable_event WHERE kind='ProjectStandardsSaved'")
            .fetch_one(reopened.reader())
            .await
            .unwrap();
    assert_eq!(count, 3);
}
