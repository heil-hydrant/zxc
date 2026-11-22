mod error;
pub mod handler;
pub mod messages;

pub use error::{HistoryError, HistoryWsError};
pub use handler::HistoryHandler;
pub use messages::{
    CommanderToHistory, HistoryUIOps, HistoryUImsg, HistoryWsRegisterInfo,
};

pub const HISTORY_WS_WSESS: &str = "history.wsess";
pub const HISTORY_WS_HIS: &str = "./ws.whis";
