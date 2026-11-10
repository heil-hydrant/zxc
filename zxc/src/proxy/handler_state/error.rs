use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

use crate::commander::*;
use crate::history::*;
use crate::io::file::FileErrorInfo;
use crate::io::socket::ConnectError;
use crate::proxy::prelude::*;

// Various Errors that can occur on handler state transition
#[derive(Error, Debug)]
pub enum ProxyStateError {
    // Communication Error
    #[error(transparent)]
    Commander(#[from] CommanderError),

    // ----- Read Write -----
    // HTTP
    #[error("http: {0}")]
    OneOne(#[from] OneOneRWError),
    #[error("h2: {0}")]
    Two(#[from] two_plz::error::OpError),
    #[error("h2 preface: {0}")]
    H2Preface(#[from] two_plz::preface::PrefaceError),
    #[error("h2 connection closed before response")]
    H2Closed,
    #[error("h2 request is missing")]
    H2MissingRequest,
    // WebSocket
    #[error("websocket: {0}")]
    Ws(#[from] WsError),

    // ----- Write History -----
    #[error("serialize: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("history channel closed")]
    HistorySend(#[from] SendError<CommanderToHistory>),

    // ----- File Ops -----
    #[error(transparent)]
    FileIo(#[from] FileErrorInfo),
    #[error(transparent)]
    ReWriteFile(FileErrorInfo),

    // ----- Should Intercept -----
    #[error("websocket should-intercept check")]
    WsShouldIntercept,

    // ----- Update Frame -----
    #[error(transparent)]
    UpdateFrame(#[from] UpdateMsgError),

    // ----- NewConnection -----
    #[error(transparent)]
    Reconnect(#[from] ConnectError),

    #[error(transparent)]
    ServerEncrypt(#[from] ServerEncryptError),
    #[error("message dropped by user")]
    Drop,
}

impl From<SendError<CommanderRequest>> for ProxyStateError {
    fn from(_: SendError<CommanderRequest>) -> Self {
        Self::Commander(CommanderError::Send)
    }
}

impl From<WrongMessage> for ProxyStateError {
    fn from(e: WrongMessage) -> Self {
        Self::Commander(CommanderError::WrongMessage(e))
    }
}
