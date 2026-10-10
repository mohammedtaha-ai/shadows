use super::*;

fn kinded(kind: &str) -> DesignOp {
    DesignOp::PartCreate {
        id: PartId::generate(),
        parent: None,
        before: None,
        content: PartContent {
            kind: Some(kind.into()),
            ..content(kind)
        },
    }
}

fn vision(users: &str) -> DesignOp {
    DesignOp::VisionPut {
        content: shadows_core::VisionContent {
            purpose: "Sell books".into(),
            users: users.into(),
            goals: "G".into(),
            boundaries: "B".into(),
            technical_direction: "Rust".into(),
        },
    }
}

fn view(stage: shadows_core::Stage, missing: &[&str]) -> shadows_core::StageView {
    shadows_core::StageView {
        stage,
        missing: missing.iter().map(|m| m.to_string()).collect(),
    }
}

#[tokio::test]
async fn the_stage_walks_idea_vision_map_structure() {
    use shadows_core::Stage;
    let tmp = tempfile::tempdir().unwrap();
    let core = core_at(&tmp.path().join("s.sqlite3")).await;
    let p = project(&core, "stage", tmp.path()).await;
    assert_eq!(
        core.design().stage(&p).await.unwrap(),
        view(Stage::Idea, &[])
    );
    edit(&core, &p, 0, vec![vision("  ")]).await;
    assert_eq!(
        core.design().stage(&p).await.unwrap(),
        view(Stage::Vision, &["users"])
    );
    edit(&core, &p, 1, vec![vision("Readers"), kinded("backend")]).await;
    assert_eq!(
        core.design().stage(&p).await.unwrap(),
        view(Stage::Map, &["database", "api", "frontend"])
    );
    edit(
        &core,
        &p,
        2,
        vec![kinded("database"), kinded("api"), kinded("frontend")],
    )
    .await;
    assert_eq!(
        core.design().stage(&p).await.unwrap(),
        view(Stage::Structure, &[])
    );
}

#[tokio::test]
async fn stage_counts_a_trimmed_kind_and_no_other_case() {
    let tmp = tempfile::tempdir().unwrap();
    let core = core_at(&tmp.path().join("s.sqlite3")).await;
    let p = project(&core, "kinds", tmp.path()).await;
    let parent = PartId::generate();
    edit(
        &core,
        &p,
        0,
        vec![
            vision("Readers"),
            kinded(" backend "),
            DesignOp::PartCreate {
                id: parent.clone(),
                parent: None,
                before: None,
                content: PartContent {
                    kind: Some("Database".into()),
                    ..content("parent")
                },
            },
            DesignOp::PartCreate {
                id: PartId::generate(),
                parent: Some(parent),
                before: None,
                content: PartContent {
                    kind: Some("database".into()),
                    ..content("nested")
                },
            },
        ],
    )
    .await;
    assert_eq!(
        core.design().stage(&p).await.unwrap().missing,
        ["database", "api", "frontend"]
    );
}

#[tokio::test]
async fn an_added_part_is_mandatory_for_the_stage() {
    let tmp = tempfile::tempdir().unwrap();
    let core = core_at(&tmp.path().join("s.sqlite3")).await;
    let p = project(&core, "added", tmp.path()).await;
    let parts = ["backend", "database", "api", "frontend"].map(kinded);
    edit(
        &core,
        &p,
        0,
        [vec![vision("Readers")], parts.to_vec()].concat(),
    )
    .await;
    let content = shadows_core::StandardsAdditions {
        parts: vec![shadows_core::AdditionalPart {
            name: "billing".into(),
            owns: "x".into(),
        }],
        rules: vec![],
    };
    core.design()
        .save_standards_additions("b".into(), &p, content)
        .await
        .unwrap();
    assert_eq!(
        core.design().stage(&p).await.unwrap(),
        view(shadows_core::Stage::Map, &["billing"])
    );
}
