use thiserror::Error;
use tokio::sync::mpsc::error::SendError;
use two_plz::codec::UserError;
use two_plz::error::OpError;
use two_plz::preface::PrefaceError;

use crate::commander::*;
use crate::proxy::prelude::*;

#[derive(Debug, Error)]
pub enum Http2Error {
    #[error("{0}")]
    Op(#[from] OpError),
    #[error("preface: {0}")]
    Preface(#[from] PrefaceError),
    #[error("connection closed before response")]
    Closed,
    #[error("client connection is missing")]
    MissingClientConnection,
    #[error("response future is missing")]
    MissingResponseFuture,
    #[error("request is missing")]
    MissingRequest,
    #[error("stream is missing")]
    MissingStream,
    #[error("user: {0}")]
    User(#[from] UserError),
    // Boxed to break the ProxyStateError -> ProtocolError -> Http2Error cycle.
    #[error(transparent)]
    Proxy(Box<ProxyStateError>),
    #[error(transparent)]
    Commander(#[from] CommanderError),
    #[error("unexpected request state")]
    UnexpectedState,
}

impl From<ProxyStateError> for Http2Error {
    fn from(e: ProxyStateError) -> Self {
        Self::Proxy(Box::new(e))
    }
}

impl From<SendError<CommanderRequest>> for Http2Error {
    fn from(_: SendError<CommanderRequest>) -> Self {
        Self::Commander(CommanderError::Send)
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for Http2Error {
    fn from(_: tokio::sync::oneshot::error::RecvError) -> Self {
        Self::Commander(CommanderError::Recv)
    }
}
