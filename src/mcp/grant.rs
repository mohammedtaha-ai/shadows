//! One job: who may do what on `/mcp` (spec §13.7).

use crate::id::newtype_id;

newtype_id! {
    /// Spec §13.7. A thread grant (the internal Planner) or a project grant
    /// (an external agent); its token is stored only as a hash.
    GrantId
}
