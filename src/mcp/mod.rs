//! One job: Shadows' MCP server (spec §13.6), served at `/mcp`.
//!
//! So far only its grants exist (§13.7): who holds one, and the token each is
//! answered by. Storage issues, revokes and checks them; the server itself
//! comes with the task that first needs it.

pub mod grant;
