use std::io;

use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

use crate::history::HistoryUIOps;
use crate::id::SoldierId;
use crate::io::UnixSockError;
use crate::run::boundary::IsUIError;

#[derive(Debug, Error)]
pub enum HistoryError {
    // UI
    #[error("ui error| {0}")]
    UI(#[from] UnixSockError),
    #[error("message decode| {0}")]
    MsgDecode(#[from] serde_json::Error),
    #[error("sending")]
    CommanderSend(#[from] SendError<HistoryUIOps>),
    // Ws
    #[error("create ws| {0}")]
    CreateWs(io::Error),
    #[error("write ws| {0}")]
    WsWrite(io::Error),
    #[error("register ws| {0}")]
    RegisterWs(io::Error),

    #[error("no id| {0} | {1}")]
    NoId(SoldierId, &'static str),

    #[error("needs flush")]
    NeedsFlush,
}

impl IsUIError for HistoryError {
    fn is_ui_error(&self) -> bool {
        matches!(self, HistoryError::UI(_))
    }

    fn needs_flush(&self) -> bool {
        matches!(self, HistoryError::NeedsFlush)
    }
}
