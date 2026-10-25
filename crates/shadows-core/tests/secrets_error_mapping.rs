use shadows_core::{CoreError, SecretsError, StorageError};

#[test]
fn secrets_errors_keep_application_codes_and_messages() {
    let cases = [
        (
            SecretsError::NameReserved("A".into()),
            CoreError::SecretNameReserved("A".into()),
        ),
        (
            SecretsError::NameInvalid("A".into()),
            CoreError::SecretNameInvalid("A".into()),
        ),
        (
            SecretsError::ValueInvalid("A".into()),
            CoreError::SecretValueInvalid("A".into()),
        ),
        (
            SecretsError::Unreadable("A".into()),
            CoreError::SecretUnreadable("A".into()),
        ),
        (SecretsError::Unavailable, CoreError::SecretsUnavailable),
        (
            SecretsError::Storage(StorageError::CommandConflict),
            CoreError::Storage(StorageError::CommandConflict),
        ),
        (
            SecretsError::Storage(StorageError::Constraint("fixture".into())),
            CoreError::Storage(StorageError::Constraint("fixture".into())),
        ),
    ];
    for (service, expected) in cases {
        let application = CoreError::from(service);
        assert_eq!(application.code(), expected.code());
        assert_eq!(application.to_string(), expected.to_string());
    }
    assert!(shadows_core::validate_env_name("API_KEY").is_ok());
    let invalid = shadows_core::validate_env_name("bad").unwrap_err();
    assert_eq!(
        invalid.code(),
        CoreError::SecretNameInvalid("bad".into()).code()
    );
    assert_eq!(
        invalid.to_string(),
        "invalid environment variable name: bad"
    );
    let opening = CoreError::from(shadows_core::OpenError::from(SecretsError::Unavailable));
    assert_eq!(opening.code(), CoreError::SecretsUnavailable.code());
    assert_eq!(
        opening.to_string(),
        CoreError::SecretsUnavailable.to_string()
    );
}
