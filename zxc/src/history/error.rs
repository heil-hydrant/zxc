use std::io;

use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

use crate::commander::CommanderError;
use crate::history::HistoryUIOps;
use crate::id::SoldierId;
use crate::run::boundary::UiError;

#[derive(Debug, Error)]
pub enum HistoryWsError {
    #[error("history file create: {0}")]
    FileCreate(#[source] io::Error),
    #[error("history write: {0}")]
    Write(#[source] io::Error),
    #[error("session register: {0}")]
    Register(#[source] io::Error),
}

#[derive(Debug, Error)]
pub enum HistoryError {
    // UI
    #[error(transparent)]
    Ui(#[from] UiError),
    #[error(transparent)]
    Commander(#[from] CommanderError),
    // Ws
    #[error("websocket: {0}")]
    Ws(#[from] HistoryWsError),

    #[error("no history entry for {0} ({1})")]
    NoId(SoldierId, &'static str),
}

crate::impl_ui_error!(HistoryError);

impl From<SendError<HistoryUIOps>> for HistoryError {
    fn from(_: SendError<HistoryUIOps>) -> Self {
        Self::Commander(CommanderError::Send)
    }
}
