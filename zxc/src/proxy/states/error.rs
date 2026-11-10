use std::convert::Infallible;
use std::io::{self, ErrorKind};

use rustls_pki_types::InvalidDnsNameError;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::oneshot::error::RecvError;

use crate::commander::*;
use crate::io::socket::ConnectError;
use crate::proxy::prelude::*;

#[derive(Error, Debug)]
pub enum StateError {
    // ----- Read Initial Client Data -----
    #[error("http: {0}")]
    InitialRead(#[from] OneOneRWError),

    // ----- Determine Server -----
    #[error(transparent)]
    Address(#[from] AddressError),

    // ----- Establish Conn -----
    #[error(transparent)]
    ServerConnect(#[from] ConnectError),

    // Proxy Write
    #[error("proxy established response write: {0}")]
    ClientWrite(#[source] io::Error),
    // Server Write
    #[error("server establishment write: {0}")]
    ServerWrite(#[source] io::Error),

    // ----- Client Encrypt -----
    // Client Handshake
    #[error("client handshake: {0}")]
    ClientHandshake(#[source] io::Error),

    // ----- Server Encrypt -----
    #[error(transparent)]
    InvalidDns(#[from] InvalidDnsNameError),
    #[error(transparent)]
    ServerEncrypt(#[from] ServerEncryptError),

    // ----- Complete Handshake -----
    #[error("no peer certificate")]
    NoPeerCertificate,
    #[error(transparent)]
    ClientCertificateGen(#[from] CertError),
    #[error("client handshake completion: {0}")]
    ClientEncrypt(#[source] io::Error),

    // ----- Protocol Handler -----
    #[error(transparent)]
    Handler(#[from] ProxyStateError),

    // ----- Communicate -----
    #[error(transparent)]
    Commander(#[from] CommanderError),
}

impl From<RecvError> for StateError {
    fn from(_: RecvError) -> Self {
        Self::Commander(CommanderError::Recv)
    }
}

impl From<SendError<CommanderRequest>> for StateError {
    fn from(_: SendError<CommanderRequest>) -> Self {
        Self::Commander(CommanderError::Send)
    }
}

impl From<WrongMessage> for StateError {
    fn from(e: WrongMessage) -> Self {
        Self::Commander(CommanderError::WrongMessage(e))
    }
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
