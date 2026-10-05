//! Grant-scoped agreement reads and proposals; no approval tool.
use super::{
    refusal::{Refusal, answer},
    server::Shadows,
};
use rmcp::{
    handler::server::{tool::Extension, wrapper::Parameters},
    model::CallToolResult,
    tool, tool_router,
};
use shadows_core::{AgreementContent, AgreementId, Grant};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct Read {
    #[schemars(with = "String")]
    agreement_id: AgreementId,
    version: Option<i64>,
}
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct Start {
    command_id: String,
    #[schemars(with = "Option<String>")]
    agreement_id: Option<AgreementId>,
    content: Option<AgreementContent>,
    reason: Option<String>,
}
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct Edit {
    command_id: String,
    #[schemars(with = "String")]
    agreement_id: AgreementId,
    /// The Draft version the edit was built on; another version is refused.
    version: i64,
    expected_revision: i64,
    content: AgreementContent,
}
#[tool_router(router=agreement_tool_router,vis="pub(super)")]
impl Shadows {
    #[tool(description = "List this project's shared API agreements and latest versions.")]
    async fn agreement_list(&self, Extension(grant): Extension<Grant>) -> CallToolResult {
        answer(
            self.core
                .design()
                .agreements_for(&grant)
                .await
                .map_err(Refusal::from),
        )
    }
    #[tool(
        description = "Read an exact agreement version; omitted version reads the current version."
    )]
    async fn agreement_get(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<Read>,
    ) -> CallToolResult {
        answer(
            self.core
                .design()
                .agreement_for(&grant, &args.agreement_id, args.version)
                .await
                .map_err(Refusal::from),
        )
    }
    #[tool(
        description = "Start/open a Draft agreement. Continuing an Agreed version needs a \
            reason. Shadows gives each operation its x-shadows-operation-id; do not \
            invent one."
    )]
    async fn agreement_start(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<Start>,
    ) -> CallToolResult {
        answer(
            self.core
                .design()
                .start_agreement_for(
                    &grant,
                    args.command_id,
                    args.agreement_id.as_ref(),
                    args.content,
                    args.reason,
                )
                .await
                .map_err(Refusal::from),
        )
    }
    #[tool(
        description = "Edit the named Draft version under expected_revision; no automatic \
            adoption. A new operation gets its x-shadows-operation-id from Shadows; keep \
            existing ones."
    )]
    async fn agreement_edit(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<Edit>,
    ) -> CallToolResult {
        answer(
            self.core
                .design()
                .edit_agreement_for(
                    &grant,
                    args.command_id,
                    &args.agreement_id,
                    args.version,
                    args.expected_revision,
                    args.content,
                )
                .await
                .map_err(Refusal::from),
        )
    }
    #[tool(description = "Read changes and participants. Compatibility needs review.")]
    async fn agreement_review(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<Read>,
    ) -> CallToolResult {
        answer(
            self.core
                .design()
                .review_agreement_for(&grant, &args.agreement_id)
                .await
                .map_err(Refusal::from),
        )
    }
}
