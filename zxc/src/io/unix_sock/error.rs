use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum UnixSockError {
    #[error("accept: {0}")]
    Accept(#[source] io::Error),
    #[error("closed")]
    Closed,
    #[error("end of file")]
    EoF,
    #[error("read: {0}")]
    Read(#[source] io::Error),
    #[error("readiness check: {0}")]
    Readable(#[source] io::Error),
    #[error("write: {0}")]
    Write(#[source] io::Error),
    #[error("would block")]
    Block,
}
