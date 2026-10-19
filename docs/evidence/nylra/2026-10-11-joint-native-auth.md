# Native authentication consumer and joint Docker evidence

Date: 2026-10-11, Asia/Aden. Baseline `0a852d0`, branch `codex/windows-shell-n0`;
clean before work. The user authorized implementing/testing the integration,
fixing both repositories, updating documentation and committing each locally.

## Implemented slice

Main-only React actions call a native fixed action set; the serialized View contains
state, enrollment presence, a verified principal and truthful provider pending status.
No session handle, ticket, verifier, enrollment credential or provider token enters
webview IPC. Native Rust consumes backend auth-v1 1.1.0 DTOs and its pinned safe
fixtures. reqwest 0.12.28 / serde 1.0.229 added to the coherent Tauri lockfile.
The reviewed local auth path is fixed to shadows-local; production issuer selection
is a separate gate. HTTPS verifies the chain/hostname using the explicit public
development CA, refuses redirects, limits response bytes and bounds each request
to 15 seconds. The CA is configured by NYLRA_DEV_CA only for loopback development.

Login binds S256 to the real ephemeral IPv4 listener and redeems one ticket.
DPAPI atomically protects an origin-bound account envelope in app-data; enrollment
and session have independent lifetimes. The primary action reuses an existing
enrollment; failures never silently enroll anew. Explicit new-registration action
has a UI confirmation. Startup/current session checks need a fresh backend response
before account identity is shown. Refused or unavailable verification preserves
credentials and exposes no principal. A failed initial status disables Sign in
until a successful local account projection, preventing accidental new enrollment.

Logout clears the handle only after confirmed backend surrender and retains
enrollment; provider completion stays pending after browser handoff. Device revoke
uses its encrypted stored target directly, letting backend local authorization
surrender authority even during provider outage; only successful revoke clears
enrollment. Cancellation is signaled separately from the blocking worker, drops
the listener, bounds in-flight HTTP and attempts to revoke an already issued session.

## Executed evidence

- Three new wire tests first failed against stubs, then passed with exact-ticket,
  reviewed-origin/path and opaque field validation. An additional fixture test
  deserializes the backend's safe auth-v1 1.1.0 examples and rejects extra fields.
- A real DPAPI plus loopback HTTP fault fixture passes refusal (401), outage (503)
  and later recovered validation, retaining enrollment/session without cached identity.
- Device-revoke fault regression first failed `backend_unavailable` because the
  client unnecessarily called session validation. Direct backend surrender then
  passed while its fixture refused ordinary GET validation. Backend still decides
  authorization; the stored device id is never an authenticated principal.
- Native suite: 17 pass, one explicit Compose test normally ignored; no failures.
  The Compose test was separately opted in and passed, including real HTTPS,
  real Windows TCP loopback return, encrypted store, reauthentication preserving
  device id, distinct session ids, revoke/new enrollment and logout.
- `auth_state` verifies the encrypted store in a fresh Windows process, checking
  the live backend before returning a safe projection. That assertion passed.
- Frontend TypeScript/Vite build passed. Rust build and strict Clippy were run;
  Clippy initially found an existing fixed-chunk warning in the tray-icon loop,
  corrected without changing its pixels.
- Sibling Docker root suite passed 42/42. Its rebuilt persistent Compose journey
  passed. See [backend evidence](../../../../shadows-backend/docs/evidence/2026-10-11-joint-native-auth.md).

Commands from NYLRA root:

```powershell
npm run build --prefix apps/desktop
cargo test --locked --manifest-path apps/desktop/src-tauri/Cargo.toml
cargo clippy --locked --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings
cargo build --locked --manifest-path apps/desktop/src-tauri/Cargo.toml --example auth_state
$env:NYLRA_DEV_CA=(Resolve-Path '..\shadows-backend\.local-runtime\tls\ca.pem').Path
$env:NYLRA_ACCEPTANCE_RUNTIME=(Resolve-Path '..\shadows-backend\.local-runtime').Path
$env:NYLRA_ACCEPTANCE_CHILD=(Resolve-Path 'apps/desktop/src-tauri/target/debug/examples/auth_state.exe').Path
cargo test --locked --manifest-path apps/desktop/src-tauri/Cargo.toml native_compose -- --ignored
```

No model requests, sibling project-rule changes, real user identities or deployment.
The acceptance driver reads only the synthetic local password in memory, never
prints it, and removes its dedicated encrypted acceptance store on success.

## Browser/app distinction and remaining acceptance

The real NYLRA window started; its account controls and signed-out state were
inspected. Clicking Sign in reached the real system browser and changed the panel
to browser-login pending. A text-only click initially lacked geometry; native
window reobservation with geometry corrected it. Computer Use then stopped because
the current Firefox browser lacked URL policy support. It was not replaced with
another UI automation path. With user permission, only the current local CA was
installed in CurrentUser Root. Firefox still displayed a trust warning; the user
reported manually bypassing it, but did not confirm a completed login. This is
not verified clean browser trust or full real app UI acceptance.

The later authenticated native test uses an HTTP form driver against actual
Keycloak rather than browser UI; its fresh-process and DPAPI results are genuine
native/backend evidence. Full app UI login/restart/cancellation/logout remains OPEN
until exercised with a correctly trusted browser. Existing AYLUL asset/render,
mixed-monitor, mobile, voice and Production gates remain unchanged. Installer,
clean-machine distribution and production issuer/credential lifecycle are not tested.
Explicit process-memory zeroization and provider-completion polling are not implemented.

Final checks passed: strict Clippy, Rust formatting, frontend build, backend-fixture
equality, five changed source-contract YAML parses and 32 local/sibling document
targets. A contract checker initially rejected a compound test reference in the
backend; it was corrected to the actual standalone regression name and passed.
After splitting policy tests out of service.rs, formatting found a leading blank
line; formatting was applied and its final check passed. service.rs retains native
account workflows, policy_tests.rs the fault regression, acceptance.rs the real
synthetic Compose journey. No unrelated formatting was performed.

The user requested another UI attempt. The final debug app was launched again,
but Computer Use then reported a physical Escape stop; no further Computer Use
was attempted afterward. The restarted app is left available for the user's manual
trial. This does not change the OPEN full browser/UI gate or the passing native
library results. The user's clipboard no longer contains the test password.
