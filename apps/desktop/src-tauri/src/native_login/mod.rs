//! NYLRA-owned Windows login primitives only. No backend HTTP adapter, identity rules or IPC.
mod browser;
mod loopback;
mod protected_store;
mod random;

pub use browser::open_system_browser;
pub use loopback::{NativeReturn, ReturnBinding};
pub use protected_store::{clear_session, load_session, save_session};
