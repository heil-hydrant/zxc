use std::io;
use std::path::PathBuf;

use header_plz::InfoLineError;
use header_plz::status::InvalidStatusCode;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

use crate::commander::CommanderError;
use crate::commander::codec::error::CodecError;
use crate::forward_info::ForwardInfo;
use crate::io::*;
use crate::proxy::server_info::address::error::AddressError;
use crate::proxy::*;
use crate::repeater::ws::*;
use crate::run::boundary::UiError;

#[derive(Debug, Error)]
pub enum RepeaterError {
    // ----- Handle Commander -----
    #[error(transparent)]
    Commander(#[from] CommanderError),
    #[error("repeater file has no extension: {0}")]
    NoExtension(PathBuf),
    #[error("unknown repeater file extension: {0}")]
    UnknownExtension(PathBuf),

    // ----- UI -----
    #[error(transparent)]
    Ui(#[from] UiError),

    // ----- Codec -----
    #[error(transparent)]
    Codec(#[from] CodecError),

    // ----- Initial State Machine -----
    // Invalid Dns
    #[error(transparent)]
    InvalidAddress(#[from] AddressError),

    // Establishing Server Connection
    #[error(transparent)]
    Connection(#[from] ConnectError),

    // Encryption
    #[error("encryption: {0}")]
    Encrypt(#[source] io::Error),
    #[error("h2 replay requires TLS")]
    H2RequiresTls,
    #[error("ALPN mismatch: expected {expected}, negotiated {negotiated:?}")]
    UnexpectedAlpn {
        expected: &'static str,
        negotiated: Option<String>,
    },
    #[error("h2 websocket replay is not supported")]
    H2WsUnsupported,

    // ----- Main State Errors -----
    #[error(transparent)]
    Proxy(#[from] ProxyStateError),

    // ----- Get status code -----
    #[error(transparent)]
    InfoLine(#[from] InfoLineError),
    #[error(transparent)]
    StatusCode(#[from] InvalidStatusCode),

    // ----- Ws -----
    // send
    #[error("websocket commander send")]
    WsSend(#[from] SendError<RepeaterWsMsg>),
    // wrong scode
    #[error("unexpected websocket status code: {0}")]
    WsWrongScode(u16),

    // Build
    #[error(transparent)]
    DirError(#[from] DirError),

    // ws build
    #[error(transparent)]
    WsBuild(#[from] RWsBuildError),
    #[error("scratch file error: {0}")]
    ScratchFile(#[source] io::Error),
    #[error("history file error: {0}")]
    HistoryFile(#[source] io::Error),
    #[error("websocket id not found: {0}")]
    WsIdNotFound(usize),
}

crate::impl_ui_error!(RepeaterError);

impl From<SendError<ForwardInfo>> for RepeaterError {
    fn from(_: SendError<ForwardInfo>) -> Self {
        Self::Commander(CommanderError::Send)
    }
}
