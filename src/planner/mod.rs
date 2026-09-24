//! Planner turns run as ACP prompts on a thread's managed adapter (§12.3).
//! `spawn` registers before Running. `turn` arbitrates terminal state with Stop.

mod entries;
mod handles;
mod sessions;
mod shutdown;
mod spawn;
mod turn;

pub use handles::LiveHandles;
pub(crate) use handles::LiveTurn;
pub use sessions::{OpenError, OpenSession, Sessions, SessionsConfig};
pub use shutdown::shut_down;
pub use spawn::{PlannerTurnRequest, StartError};
pub use turn::{PlannerTurn, StopOutcome};
