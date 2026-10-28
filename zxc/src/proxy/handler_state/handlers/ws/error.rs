use thiserror::Error;
use tokio::sync::mpsc::error::SendError;
use tokio_tungstenite::tungstenite;

use crate::commander::*;
use crate::history::*;

#[derive(Debug, Error)]
pub enum WsCreationError {
    #[error("create dir| {0}")]
    CreateDir(#[from] std::io::Error),
    #[error("send| {0}")]
    Send(#[from] SendError<CommanderRequest>),
    #[error("No Reply")]
    NoReply,
    #[error("wrong command| {0}")]
    WrongCommand(#[from] WrongMessage),
    #[error("history send| {0}")]
    HistorySend(#[from] SendError<CommanderToHistory>),
}

/* Description:
 *      Error common to ws handlers.
 *
 * Used In:
 *      ReadWrite trait implementation by
 *          - wstruct/impl_read_write.rs (proxy ws handler)
 *          - rwstruct/impl_read_write.rs (repeater ws handler)
 *
 */
#[derive(Debug, Error)]
pub enum WsError {
    #[error("creation| {0}")]
    Create(#[from] WsCreationError),
    #[error("read| {0}")]
    Read(#[from] tungstenite::Error),
    #[error("write")]
    Write,
    #[error("close")]
    Close,
}
