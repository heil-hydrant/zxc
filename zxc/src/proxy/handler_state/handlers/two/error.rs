use thiserror::Error;
use tokio::sync::mpsc::error::SendError;
use two_plz::codec::UserError;
use two_plz::error::OpError;
use two_plz::preface::PrefaceError;

use crate::commander::*;
use crate::proxy::prelude::*;

#[derive(Debug, Error)]
pub(crate) enum Http2Error {
    #[error("preface: {0}")]
    Preface(#[from] PrefaceError),
    #[error("operation: {0}")]
    Op(#[from] OpError),
    #[error("user: {0}")]
    User(#[from] UserError),
    #[error(transparent)]
    Proxy(#[from] ProxyStateError),
    #[error(transparent)]
    Commander(#[from] CommanderError),
    #[error("unexpected request state")]
    UnexpectedState,
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
