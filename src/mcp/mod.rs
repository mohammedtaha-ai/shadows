//! One job: Shadows' MCP server (spec §13.6), served at `/mcp`.
//!
//! Only the grant's identity exists so far: storage checks a writer's grant
//! inside every plan write (§13.7). Issuing grants and the server itself come
//! with the tasks that first need them.

// `GrantId::from_stored` is the one id reader nothing calls yet: storage
// reads a grant back first in B5, which looks a grant up by its token. An
// `expect`, not an `allow`, so the first caller turns it into an error and it
// cannot outlive its reason.
#[expect(
    dead_code,
    reason = "GrantId::from_stored has no caller until grants are read back (B5)"
)]
pub mod grant;
