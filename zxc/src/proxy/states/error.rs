use std::convert::Infallible;
use std::io::{self, ErrorKind};

use rustls_pki_types::InvalidDnsNameError;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::oneshot::error::RecvError;

use super::connection::encrypt::ServerEncryptError;
use crate::CommanderRequest;
use crate::commander::captain_crypto::error::CertError;
use crate::commander::communicate::response::convert::WrongMessage;
use crate::io::socket::ConnectError;
use crate::proxy::handler_state::error::ProxyStateError;
use crate::proxy::handler_state::handlers::oneonestruct::OneOneRWError;
use crate::proxy::server_info::address::error::AddressError;

#[derive(Error, Debug)]
pub enum StateError {
    // ----- Read Initial Client Data -----
    #[error("initial read| {0}")]
    InitialRead(#[from] OneOneRWError),

    // ----- Determine Server -----
    #[error("parse address| {0}")]
    Address(#[from] AddressError),

    // ----- Establish Conn -----
    #[error("connect| {0}")]
    ServerConnect(#[from] ConnectError),

    // ---- Should Proxy -----
    #[error("should proxy| {0}")]
    ShouldProxy(#[from] RecvError),

    // Proxy Write
    #[error("writing proxy established| {0}")]
    ClientWrite(io::Error),
    // Server Write
    #[error("writing server establishment| {0}")]
    ServerWrite(io::Error),

    // ----- Client Encrypt -----
    // Client Handshake
    #[error("client handshake| {0}")]
    ClientHandshake(io::Error),

    // ----- Server Encrypt -----
    #[error("invalid dns| {0}")]
    InvalidDns(#[from] InvalidDnsNameError),
    #[error("server encrypt| {0}")]
    ServerEncrypt(#[from] ServerEncryptError),

    // ----- Complete Handshake -----
    #[error("no peer certificate")]
    NoPeerCertificate,
    #[error("client cert gen| {0}")]
    ClientCertificateGen(#[from] CertError),
    #[error("complete handshake| {0}")]
    ClientEncrypt(io::Error),

    // ----- Protocol Handler -----
    #[error("handler| {0}")]
    Handler(#[from] ProxyStateError),

    // ----- Communicate -----
    #[error("wrong response| {0}")]
    WrongMessage(#[from] WrongMessage),
    // mpsc sender error
    #[error("commander send| {0}")]
    CommanderSend(#[from] SendError<CommanderRequest>),
    // mpsc recvr
    #[error("commander recv| {0}")]
    CommanderRecv(&'static str),
}

impl StateError {
    pub fn is_common_error(&self) -> bool {
        match self {
            Self::ClientEncrypt(e) | Self::ClientHandshake(e) => {
                e.kind() == ErrorKind::UnexpectedEof
            }
            Self::ServerEncrypt(ServerEncryptError::Io(e)) => {
                e.kind() == ErrorKind::ConnectionReset
            }
            Self::Handler(e) => {
                matches!(e, ProxyStateError::Drop)
            }
            _ => false,
        }
    }
}

impl From<Infallible> for StateError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}
