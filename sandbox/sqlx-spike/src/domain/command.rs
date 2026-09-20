use serde::{Deserialize, Serialize};

use super::ids::{CommandId, OperationId, Principal, Timestamp};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandRecord {
    pub command_id: CommandId,
    pub principal: Principal,
    pub scope: String,
    pub operation_id: OperationId,
    pub recorded_at: Timestamp,
}
