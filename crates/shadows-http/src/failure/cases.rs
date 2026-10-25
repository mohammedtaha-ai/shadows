use shadows_core::{CoreError, DirectoryError, ErrorCode, StartError, StorageError};

struct Case {
    error: CoreError,
    code: ErrorCode,
    status: u16,
    http_code: ErrorCode,
    http: &'static str,
    mcp_code: ErrorCode,
    mcp: &'static str,
}

impl Case {
    fn same(error: CoreError, code: ErrorCode, status: u16, message: &'static str) -> Self {
        Self {
            error,
            code,
            status,
            http_code: code,
            http: message,
            mcp_code: code,
            mcp: message,
        }
    }

    fn mcp(mut self, code: ErrorCode, message: &'static str) -> Self {
        self.mcp_code = code;
        self.mcp = message;
        self
    }

    fn http_code(mut self, code: ErrorCode) -> Self {
        self.http_code = code;
        self
    }

    fn all() -> Vec<Self> {
        use CoreError as C;
        use ErrorCode as E;
        let database = "the daemon could not read or write its database";
        let mut cases = vec![
            Self::same(
                C::ValidationFailed {
                    message: "internal".into(),
                    problems: vec!["first".into(), "second".into()],
                },
                E::WorkflowValidationFailed,
                422,
                "first; second",
            ),
            Self::same(
                C::ReasonMissing,
                E::StorageUnavailable,
                500,
                "storage is unavailable",
            )
            .mcp(E::StorageUnavailable, database),
            Self::same(
                C::Unavailable("internal".into()),
                E::StorageUnavailable,
                500,
                "storage is unavailable",
            )
            .mcp(E::StorageUnavailable, database),
            Self::same(
                C::ExtensionNotFound,
                E::ExtensionNotFound,
                404,
                "extension not found",
            ),
            Self::same(
                C::ExtensionNameConflict,
                E::ExtensionNameConflict,
                409,
                "extension name is already in use",
            ),
            Self::same(
                C::AylulProfileNotFound("x".into()),
                E::AylulProfileNotFound,
                404,
                "AYLUL agent not found: x",
            ),
            Self::same(
                C::AylulProviderNotFound("x".into()),
                E::AylulProviderNotFound,
                404,
                "AYLUL provider not found: x",
            ),
            Self::same(
                C::ProviderInUse("x".into()),
                E::ProviderInUse,
                409,
                "provider is used by: x",
            ),
            Self::same(
                C::ProfileInvalid("x".into()),
                E::ProfileInvalid,
                400,
                "invalid profile: x",
            ),
            Self::same(
                C::SecretNameReserved("x".into()),
                E::SecretNameReserved,
                409,
                "reserved environment name: x",
            ),
            Self::same(
                C::SecretNameInvalid("x".into()),
                E::SecretNameInvalid,
                400,
                "invalid environment variable name: x",
            ),
            Self::same(
                C::SecretValueInvalid("x".into()),
                E::SecretValueInvalid,
                400,
                "invalid environment variable value for x",
            ),
            Self::same(
                C::SecretUnreadable("x".into()),
                E::SecretUnreadable,
                409,
                "cannot decrypt x; replace it in Settings",
            ),
            Self::same(
                C::SecretsUnavailable,
                E::SecretsUnavailable,
                503,
                "encrypted environment storage is unavailable",
            ),
            Self::same(
                C::AylulKeyMissing("x".into()),
                E::AylulKeyMissing,
                409,
                "AYLUL key is missing: x",
            ),
            Self::same(
                C::AylulNotConfigured,
                E::AylulNotConfigured,
                404,
                "AYLUL is not configured",
            ),
            Self::same(
                C::ProjectDirectoryUnusable("x".into()),
                E::PathNotFound,
                409,
                "x",
            ),
            Self::same(
                C::HarnessUnavailable("x".into()),
                E::HarnessUnavailable,
                422,
                "the x harness is not available yet",
            ),
            Self::same(
                C::ModeNotAllowed("x".into()),
                E::ModeNotAllowed,
                403,
                "this project does not allow the x mode",
            ),
            Self::same(
                C::HarnessStartFailed("x".into()),
                E::HarnessStartFailed,
                502,
                "the harness could not start",
            )
            .mcp(E::HarnessStartFailed, "the harness could not start: x"),
            Self::same(
                C::SettingNotOffered {
                    what: "model".into(),
                    id: "x".into(),
                    detail: None,
                },
                E::SettingNotOffered,
                422,
                "model x is not offered",
            ),
            Self::same(
                C::SettingNotOffered {
                    what: "model".into(),
                    id: "x".into(),
                    detail: Some("reason".into()),
                },
                E::SettingNotOffered,
                422,
                "model x was refused by the harness: reason",
            )
            .mcp(E::SettingNotOffered, "model x is not offered"),
            Self::same(
                C::RuntimeStopping,
                E::RuntimeStopping,
                503,
                "the runtime is stopping and accepts no new turns",
            )
            .mcp(E::RuntimeStopping, "the daemon is stopping"),
            Self::same(
                C::Start(StartError::RuntimeStopping),
                E::RuntimeStopping,
                503,
                "the runtime is stopping and accepts no new turns",
            ),
            Self::same(
                C::TerminationFailed,
                E::ProcessTerminationFailed,
                500,
                "the turn's process tree could not be terminated; it was not recorded as cancelled",
            )
            .mcp(
                E::ProcessTerminationFailed,
                "the turn's process tree could not be terminated",
            ),
        ];
        for wrap in [C::Storage, |error| C::Start(StartError::Storage(error))] {
            cases.extend([
                Self::same(wrap(StorageError::Unavailable("internal".into())),
                    E::StorageUnavailable, 500, "storage is unavailable")
                    .mcp(E::StorageUnavailable, database),
                Self::same(wrap(StorageError::Constraint("x".into())),
                    E::StorageConstraintViolation, 409,
                    "the request conflicts with what is already stored, such as a slug \
                     already in use")
                    .mcp(E::InvalidCommand, "x"),
                Self::same(wrap(StorageError::NotFound("x")), E::InvalidCommand,
                    404, "not found: x").mcp(E::InvalidCommand, "no such x"),
                Self::same(wrap(StorageError::TransitionConflict { expected: "a".into(),
                    found: "b".into() }), E::StorageConstraintViolation, 409,
                    "the operation had already moved on; read it again")
                    .mcp(E::StorageUnavailable, database),
                Self::same(wrap(StorageError::CommandConflict), E::CommandConflict, 409,
                    "command conflict: the same command id was reused with a different request")
                    .mcp(E::CommandConflict,
                        "this command id, or this draft_ref, already did something different; \
                         for a new plan call draft_prepare again"),
                Self::same(wrap(StorageError::RevisionConflict { current: 2,
                    summary: "x".into() }), E::RevisionConflict, 409,
                    "the resource changed; current revision is 2: x")
                    .mcp(E::RevisionConflict,
                        "the plan changed; current revision is 2: x. \
                         Read it with workflow_get, then rebuild your edit."),
                Self::same(C::Refused { code: E::ProjectHasThreads,
                    message: "the project has planning threads; it cannot be removed".into() },
                    E::ProjectHasThreads, 409,
                    "the project has planning threads; it cannot be removed"),
                Self::same(wrap(StorageError::Json(
                    serde_json::from_str::<serde_json::Value>("{").unwrap_err())),
                    E::StorageUnavailable, 500, "storage is unavailable")
                    .mcp(E::StorageUnavailable, database),
            ]);
        }
        cases.push(Self::same(
            C::Directory(DirectoryError::NotAbsolute("x".into())),
            E::PathInvalid,
            400,
            "the path must be absolute, got `x`",
        ));
        cases.push(Self::same(
            C::Directory(DirectoryError::NotUtf8),
            E::PathInvalid,
            400,
            "the path is not valid UTF-8, so it cannot be shown to a client or stored",
        ));
        cases.push(Self::same(
            C::Directory(DirectoryError::NotFound("x".into())),
            E::PathNotFound,
            404,
            "no such path: `x`",
        ));
        cases.push(Self::same(
            C::Directory(DirectoryError::NotADirectory("x".into())),
            E::PathNotADirectory,
            400,
            "not a directory: `x`",
        ));
        cases.push(Self::same(
            C::Directory(DirectoryError::AccessDenied("x".into())),
            E::PathAccessDenied,
            403,
            "access denied: `x`",
        ));
        cases.push(Self::same(
            C::Directory(DirectoryError::InvalidName {
                name: "x".into(),
                why: "reason",
            }),
            E::PathInvalid,
            400,
            "`x` is not a usable directory name: reason",
        ));
        cases.push(Self::same(
            C::Directory(DirectoryError::AlreadyExists("x".into())),
            E::PathAlreadyExists,
            409,
            "already exists: `x`",
        ));
        cases.push(Self::same(
            C::Directory(DirectoryError::Unavailable {
                path: "x".into(),
                source: std::io::Error::other("reason"),
            }),
            E::PathUnavailable,
            500,
            "`x` could not be read: reason",
        ));
        for code in [
            E::ThreadBusy,
            E::QueuedMessageGone,
            E::HarnessLocked,
            E::StageBlocked,
            E::WorkflowFrozenImmutable,
            E::InvalidCommand,
            E::GrantInvalid,
            E::GrantScope,
        ] {
            let status = match code {
                E::QueuedMessageGone => 404,
                E::ThreadBusy | E::HarnessLocked | E::StageBlocked | E::WorkflowFrozenImmutable => {
                    409
                }
                _ => 422,
            };
            let case = Self::same(
                C::Refused {
                    code,
                    message: "refusal".into(),
                },
                code,
                status,
                "refusal",
            );
            cases.push(match code {
                E::ThreadBusy | E::QueuedMessageGone => case.mcp(E::StorageUnavailable, database),
                E::WorkflowFrozenImmutable => case.mcp(
                    code,
                    "this plan version is approved and never changes again. \
                     Start a new version with draft_start.",
                ),
                _ => case,
            });
        }
        cases.push(
            Self::same(
                C::Refused {
                    code: E::GrantInvalid,
                    message: "the grant is unknown or revoked".into(),
                },
                E::GrantInvalid,
                500,
                "the daemon refused its own request",
            )
            .http_code(E::StorageUnavailable)
            .mcp(
                E::GrantInvalid,
                "this grant was revoked while the call ran; nothing was changed",
            ),
        );
        cases.push(
            Self::same(
                C::Refused {
                    code: E::GrantScope,
                    message: "outside what the grant allows".into(),
                },
                E::GrantScope,
                500,
                "the daemon refused its own request",
            )
            .http_code(E::StorageUnavailable)
            .mcp(
                E::GrantScope,
                "outside what this grant allows: plan writes stay in its own project; \
                 workflow_list and workflow_get can read an explicitly linked project; \
                 draft_ref starts a plan only within the hour after draft_prepare",
            ),
        );
        cases
    }
}
