use super::*;
use shadows_core::{ErrorCode, VisionField};

fn add(field: VisionField, text: &str) -> DesignOp {
    DesignOp::VisionItemAdd {
        field,
        text: text.into(),
    }
}

#[tokio::test]
async fn every_vision_change_is_a_stored_version() {
    let tmp = tempfile::tempdir().unwrap();
    let (core, p) = standards::core_and_project(&tmp).await;
    core.design()
        .edit(
            "a".into(),
            &p,
            0,
            vec![add(VisionField::Purpose, "Sell books")],
        )
        .await
        .unwrap();
    core.design()
        .edit(
            "b".into(),
            &p,
            1,
            vec![DesignOp::VisionItemPut {
                id: "PU1".into(),
                text: "Sell used books".into(),
            }],
        )
        .await
        .unwrap();
    let storage = Storage::open(&tmp.path().join("standards.sqlite3"))
        .await
        .unwrap();
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT number,content_json FROM design_vision_version WHERE project_id=? ORDER BY number",
    )
    .bind(p.as_str())
    .fetch_all(storage.reader())
    .await
    .unwrap();
    assert_eq!(rows.iter().map(|r| r.0).collect::<Vec<_>>(), [1, 2]);
    assert!(rows[0].1.contains("Sell books") && rows[1].1.contains("Sell used books"));
    assert_eq!(
        core.design().vision(&p).await.unwrap().content.purpose[0].revision,
        2
    );
}

#[tokio::test]
async fn a_removed_id_is_never_reused() {
    let tmp = tempfile::tempdir().unwrap();
    let (core, p) = standards::core_and_project(&tmp).await;
    core.design()
        .edit(
            "a".into(),
            &p,
            0,
            vec![
                add(VisionField::Goals, "one"),
                add(VisionField::Goals, "two"),
            ],
        )
        .await
        .unwrap();
    core.design()
        .edit(
            "r".into(),
            &p,
            1,
            vec![DesignOp::VisionItemRemove { id: "G2".into() }],
        )
        .await
        .unwrap();
    core.design()
        .edit("b".into(), &p, 2, vec![add(VisionField::Goals, "three")])
        .await
        .unwrap();
    let v = core.design().vision(&p).await.unwrap();
    assert_eq!(
        v.content
            .goals
            .iter()
            .map(|i| i.id.as_str())
            .collect::<Vec<_>>(),
        ["G1", "G3"]
    );
    core.design()
        .edit(
            "batch".into(),
            &p,
            3,
            vec![
                add(VisionField::Goals, "temporary"),
                DesignOp::VisionItemRemove { id: "G4".into() },
                add(VisionField::Goals, "last"),
            ],
        )
        .await
        .unwrap();
    assert_eq!(
        core.design().vision(&p).await.unwrap().content.goals[2].id,
        "G5"
    );
}

#[tokio::test]
async fn item_ops_refuse_an_unknown_id_and_empty_text() {
    let tmp = tempfile::tempdir().unwrap();
    let (core, p) = standards::core_and_project(&tmp).await;
    for (n, op) in [
        DesignOp::VisionItemPut {
            id: "G9".into(),
            text: "x".into(),
        },
        DesignOp::VisionItemRemove { id: "X1".into() },
        add(VisionField::Users, " \t\n\u{a0}"),
    ]
    .into_iter()
    .enumerate()
    {
        assert!(matches!(
            core.design().edit(n.to_string(), &p, 0, vec![op]).await,
            Err(CoreError::Refused {
                code: ErrorCode::InvalidCommand,
                ..
            })
        ));
    }
    assert_eq!(core.design().vision(&p).await.unwrap().revision, 0);
}
