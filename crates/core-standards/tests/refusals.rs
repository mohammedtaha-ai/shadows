use std::sync::Arc;

use core_db::{Storage, StorageError};
use core_projects::Projects;
use core_standards::{AdditionalPart, Standards, StandardsAdditions, StandardsError};

#[tokio::test]
async fn invalid_additions_write_nothing_and_command_identity_is_preserved() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let project = Projects::new(storage.clone())
        .create(
            "project".into(),
            "project",
            "Project",
            tmp.path().to_str().unwrap(),
        )
        .await
        .unwrap();
    let standards = Standards::new(storage);
    let invalid = StandardsAdditions {
        parts: vec![AdditionalPart {
            name: "backend".into(),
            owns: "duplicate".into(),
        }],
        ..Default::default()
    };
    let error = standards
        .save_standards_additions("save".into(), &project.id, invalid.clone())
        .await
        .unwrap_err();
    assert!(matches!(error, StandardsError::InvalidAdditions(_)));
    assert_eq!(
        error.to_string(),
        "part \"backend\" is already a mandatory part"
    );
    assert_eq!(standards.current(&project.id).await.unwrap(), None);
    let first = standards
        .save_standards_additions("save".into(), &project.id, Default::default())
        .await
        .unwrap();
    assert_eq!(first.number, 1);
    let replay = standards
        .save_standards_additions("save".into(), &project.id, Default::default())
        .await
        .unwrap();
    assert_eq!(first, replay);
    assert!(matches!(
        standards
            .save_standards_additions("save".into(), &project.id, invalid)
            .await,
        Err(StandardsError::Storage(StorageError::CommandConflict))
    ));
}
