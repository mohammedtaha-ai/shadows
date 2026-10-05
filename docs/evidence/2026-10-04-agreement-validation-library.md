# Shared agreements: validation library check

Date: 2026-10-04. These measurements precede Stage 2 acceptance and do not
claim that bindings, impact review or the full contract journey are complete.

Selected `jsonschema` **0.58.5**, with default features disabled. `cargo info`
reports Rust minimum **1.85.0**, below this workspace's 1.94 floor. Its default
HTTP/file resolution features are not enabled; validation uses an offline
registry with the official OpenAPI resources bundled in the product.

Primary interface: [jsonschema 0.58.5](https://docs.rs/jsonschema/0.58.5/jsonschema/).
The crate's `Registry::add`, `prepare`, `options().offline().with_registry`
and `iter_errors` were checked against its downloaded source and exercised
by the content tests.

Official schemas come from [OAI's specification website repository](https://github.com/OAI/spec.openapis.org/tree/main/oas/3.1):
schema and schema-base `2022-10-07`, dialect/base and meta/base. They describe
OAS **3.1.x**, matching the accepted interface convention. Their `$id` is
retained. Bundled copies prevent live schema changes or network retrieval
during a contract edit. [OAS 3.1.0](https://spec.openapis.org/oas/v3.1.0.html)
remains the semantic authority; schema validity alone cannot prove runtime
API behavior or backward compatibility.

The base structural schema explicitly excludes Schema Object validation.
A test with `type: "not-a-type"` passed incorrectly with it alone. Adding
schema-base and the OAS dialect/meta resources made that regression fail
then pass correctly. The final focused validation suite passes for login
content, malformed responses, duplicate/missing operation UUIDs, incomplete
intent, valid local references and unresolved local/external references.

Direct downloads from spec.openapis.org timed out on this machine; the same
official resources were retrieved through its GitHub repository. No
unofficial schema or custom schema language was substituted.
