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
