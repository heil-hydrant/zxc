//! History window backend.
//!
//! Owns HTTP and WebSocket history logging for the proxy and mirrors every
//! `CommanderToHistory` message to the Vim UI. Entries buffered while the
//! UI is disconnected stay in `ui_storage` and are saved to
//! `.history.state`.

mod error;
pub mod handler;
pub mod message;

pub use error::{HistoryError, HistoryWsError};
pub use handler::HistoryHandler;
pub use message::{
    CommanderToHistory, HistoryUIOps, HistoryUImsg, HistoryWsRegisterInfo,
};

pub const HISTORY_WS_WSESS: &str = "history.wsess";
pub const HISTORY_WS_HIS: &str = "./ws.whis";
