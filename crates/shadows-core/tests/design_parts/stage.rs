use super::*;

fn kinded(kind: &str) -> DesignOp {
    DesignOp::PartCreate {
        id: PartId::generate(),
        parent: None,
        before: None,
        content: PartContent {
            serves: vec![],
            folders: vec![],
            files: vec![],
            contract: Default::default(),
            kind: Some(kind.into()),
            ..content(kind)
        },
    }
}

pub(super) fn vision(users: &str) -> Vec<DesignOp> {
    use shadows_core::VisionField;
    [
        (VisionField::Purpose, "Sell books"),
        (VisionField::Users, users),
        (VisionField::Goals, "G"),
        (VisionField::Boundaries, "B"),
        (VisionField::TechnicalDirection, "Rust"),
    ]
    .into_iter()
    .filter(|(_, text)| !text.trim().is_empty())
    .map(|(field, text)| DesignOp::VisionItemAdd {
        field,
        text: text.into(),
    })
    .collect()
}

fn view(stage: shadows_core::Stage, missing: &[&str]) -> shadows_core::StageView {
    shadows_core::StageView {
        drift: vec![],
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
    edit(&core, &p, 0, vision("  ")).await;
    assert_eq!(
        core.design().stage(&p).await.unwrap(),
        view(Stage::Vision, &["users"])
    );
    edit(
        &core,
        &p,
        1,
        [vision("Readers"), vec![kinded("backend")]].concat(),
    )
    .await;
    assert_eq!(
        core.design().stage(&p).await.unwrap().missing,
        ["database", "api", "frontend"]
    );
    edit(
        &core,
        &p,
        2,
        vec![kinded("database"), kinded("api"), kinded("frontend")],
    )
    .await;
    assert_eq!(
        core.design().stage(&p).await.unwrap().stage,
        Stage::Structure
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
        [
            vision("Readers"),
            vec![
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
        ]
        .concat(),
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
    edit(&core, &p, 0, [vision("Readers"), parts.to_vec()].concat()).await;
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
    assert_eq!(core.design().stage(&p).await.unwrap().missing, ["billing"]);
}
