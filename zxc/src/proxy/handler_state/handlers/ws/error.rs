use thiserror::Error;
use tokio::sync::mpsc::error::SendError;
use tokio_tungstenite::tungstenite;

use crate::commander::*;
use crate::history::*;

#[derive(Debug, Error)]
pub enum WsCreationError {
    #[error("directory create: {0}")]
    CreateDir(#[from] std::io::Error),
    #[error("commander send")]
    Send(#[from] SendError<CommanderRequest>),
    #[error("commander reply missing")]
    NoReply,
    #[error(transparent)]
    WrongCommand(#[from] WrongMessage),
    #[error("history send")]
    HistorySend(#[from] SendError<CommanderToHistory>),
}

/// Errors common to the proxy and repeater WebSocket handlers.
///
/// Shared by `WsConn` and `RWebSocket`.
#[derive(Debug, Error)]
pub enum WsError {
    #[error(transparent)]
    Create(#[from] WsCreationError),
    #[error("read: {0}")]
    Read(#[from] tungstenite::Error),
    #[error("write")]
    Write,
    #[error("message missing")]
    NoMsg,
    #[error("closed")]
    Close,
}
