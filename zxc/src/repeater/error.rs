use std::io;
use std::path::PathBuf;

use header_plz::InfoLineError;
use header_plz::status::InvalidStatusCode;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

use super::ws::builder::RWsBuildError;
use super::ws::message::RepeaterWsMsg;
use crate::commander::codec::error::CodecError;
use crate::forward_info::ForwardInfo;
use crate::io::inc_dir::DirError;
use crate::io::socket::ConnectError;
use crate::io::unix_sock::error::UnixSockError;
use crate::proxy::handler_state::error::ProxyStateError;
use crate::proxy::server_info::address::error::AddressError;
use crate::run::boundary::IsUIError;

#[derive(Debug, Error)]
pub enum RepeaterError {
    // ----- Handle Commander -----
    #[error("commander send message")]
    CommanderSend(#[from] SendError<ForwardInfo>),
    #[error("no extension| {0}")]
    NoExtension(PathBuf),
    #[error("unknown extension| {0}")]
    UnknownExtension(PathBuf),
    #[error("no ui")]
    NoUI,

    // ----- Parsing message from ui -----
    #[error("Parse message| {0}")]
    MsgSerializing(#[from] serde_json::Error),

    // ----- Socket -----
    #[error("ui | {0}")]
    UI(#[from] UnixSockError),

    // ----- Codec -----
    #[error("{0}")]
    Codec(#[from] CodecError),

    // ----- Initial State Machine -----
    // Invalid Dns
    #[error("Invalid Address| {0}")]
    InvalidAddress(#[from] AddressError),

    // Establishing Server Connection
    #[error("connect| {0}")]
    Connection(#[from] ConnectError),

    // Encryption
    #[error("Encrypt| {0}")]
    Encrypt(io::Error),

    // ----- Main State Errors -----
    #[error("main state| {0}")]
    Proxy(#[from] ProxyStateError),

    // ----- Get status code -----
    #[error("{0}")]
    InfoLine(#[from] InfoLineError),
    #[error("{0}")]
    StatusCode(#[from] InvalidStatusCode),

    // ----- Ws -----
    // send
    #[error("ws commander Send| {0}")]
    WsSend(#[from] SendError<RepeaterWsMsg>),
    // wrong scode
    #[error("ws wrong scode| {0}")]
    WsWrongScode(u16),

    // Build
    #[error("Dir| {0}")]
    DirError(#[from] DirError),

    // ws build
    #[error("ws build| {0}")]
    WsBuild(#[from] RWsBuildError),
    #[error("scratch file| {0}")]
    ScratchFile(io::Error),
    #[error("history file| {0}")]
    HistoryFile(io::Error),
    #[error("ws id not found| {0}")]
    WsIdNotFound(usize),
}

impl IsUIError for RepeaterError {
    fn is_ui_error(&self) -> bool {
        matches!(self, RepeaterError::UI(_))
    }

    fn needs_flush(&self) -> bool {
        false
    }
}
