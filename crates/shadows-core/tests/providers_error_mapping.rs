use shadows_core::{CoreError, ProvidersError, SecretsError, StorageError};

#[test]
fn provider_failures_keep_application_codes_and_messages() {
    let cases = [
        (
            ProvidersError::NotFound("missing".into()),
            CoreError::AylulProviderNotFound("missing".into()),
        ),
        (
            ProvidersError::InvalidProfile("fixture".into()),
            CoreError::ProfileInvalid("fixture".into()),
        ),
        (
            ProvidersError::InvalidField {
                field: "name".into(),
                message: "Name required.".into(),
            },
            CoreError::ProfileInvalidField {
                field: "name".into(),
                message: "Name required.".into(),
            },
        ),
        (
            ProvidersError::KeyMissing("fixture".into()),
            CoreError::AylulKeyMissing("fixture".into()),
        ),
        (
            ProvidersError::Secrets(SecretsError::Unavailable),
            CoreError::SecretsUnavailable,
        ),
        (
            ProvidersError::Storage(StorageError::CommandConflict),
            CoreError::Storage(StorageError::CommandConflict),
        ),
    ];
    for (service, expected) in cases {
        let application = CoreError::from(service);
        assert_eq!(application.code(), expected.code());
        assert_eq!(application.to_string(), expected.to_string());
    }
    assert!(
        matches!(CoreError::from(ProvidersError::InvalidField { field: "models[1].id".into(), message: "bad".into() }), CoreError::ProfileInvalidField { field, message } if field == "models[1].id" && message == "bad")
    );
    let opening = CoreError::from(shadows_core::OpenError::from(ProvidersError::NotFound(
        "fixture".into(),
    )));
    assert_eq!(
        opening.code(),
        CoreError::AylulProviderNotFound("fixture".into()).code()
    );
}
