use thiserror::Error;
use tokio::sync::mpsc::error::SendError;
use two_plz::codec::UserError;
use two_plz::error::OpError;
use two_plz::preface::PrefaceError;

use crate::commander::*;
use crate::history::*;
use crate::io::file::FileErrorInfo;
use crate::io::socket::ConnectError;
use crate::proxy::prelude::*;

#[derive(Error, Debug)]
pub enum ProtocolError {
    // H11
    #[error("h1: {0}")]
    OneOne(#[from] OneOneRWError),
    // H2
    #[error("h2: {0}")]
    H2(#[from] Http2Error),
    // WebSocket
    #[error("ws: {0}")]
    Ws(#[from] WsError),

    //  File Ops
    #[error(transparent)]
    File(#[from] FileErrorInfo),

    //  Update Frame
    #[error(transparent)]
    UpdateFrame(#[from] UpdateMsgError),
}

impl From<OpError> for ProtocolError {
    fn from(e: OpError) -> Self {
        Self::H2(Http2Error::Op(e))
    }
}

impl From<PrefaceError> for ProtocolError {
    fn from(e: PrefaceError) -> Self {
        Self::H2(Http2Error::Preface(e))
    }
}

#[derive(Error, Debug)]
pub enum ProxyStateError {
    // Communication Error
    #[error(transparent)]
    Commander(#[from] CommanderError),

    //  Read Write
    #[error("h2 send response: {0}")]
    H2SendResponse(#[from] UserError),

    //  Write History
    #[error("serialize: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("history channel closed")]
    HistorySend(#[from] SendError<CommanderToHistory>),

    //  Should Intercept
    #[error("websocket should-intercept check")]
    WsShouldIntercept,

    //  NewConnection
    #[error(transparent)]
    Reconnect(#[from] ConnectError),

    #[error(transparent)]
    Address(#[from] AddressError),

    #[error(transparent)]
    ServerEncrypt(#[from] ServerEncryptError),
    #[error("message dropped by user")]
    Drop,

    //  Protocol
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
}

impl From<OneOneRWError> for ProxyStateError {
    fn from(e: OneOneRWError) -> Self {
        Self::Protocol(ProtocolError::OneOne(e))
    }
}

impl From<two_plz::error::OpError> for ProxyStateError {
    fn from(e: two_plz::error::OpError) -> Self {
        Self::Protocol(ProtocolError::H2(Http2Error::Op(e)))
    }
}

impl From<two_plz::preface::PrefaceError> for ProxyStateError {
    fn from(e: two_plz::preface::PrefaceError) -> Self {
        Self::Protocol(ProtocolError::H2(Http2Error::Preface(e)))
    }
}

impl From<Http2Error> for ProxyStateError {
    fn from(e: Http2Error) -> Self {
        Self::Protocol(ProtocolError::H2(e))
    }
}

impl From<WsError> for ProxyStateError {
    fn from(e: WsError) -> Self {
        Self::Protocol(ProtocolError::Ws(e))
    }
}

impl From<FileErrorInfo> for ProxyStateError {
    fn from(e: FileErrorInfo) -> Self {
        Self::Protocol(ProtocolError::File(e))
    }
}

impl From<UpdateMsgError> for ProxyStateError {
    fn from(e: UpdateMsgError) -> Self {
        Self::Protocol(ProtocolError::UpdateFrame(e))
    }
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
