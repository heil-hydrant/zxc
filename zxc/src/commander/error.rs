use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

use super::{CommanderResponse, WrongMessage};
use crate::config::ConfigError;
use crate::forward_info::ForwardInfo;
use crate::history::CommanderToHistory;
use crate::id::SoldierId;

// Errors shared by components that communicate with the commander.
#[derive(Debug, Error)]
pub enum CommanderError {
    #[error("commander message send")]
    Send,
    #[error("commander response channel closed")]
    Recv,
    #[error("commander response channel closed: {0}")]
    RecvContext(&'static str),
    #[error(transparent)]
    WrongMessage(#[from] WrongMessage),
}

impl<T> From<SendError<T>> for CommanderError {
    fn from(_: SendError<T>) -> Self {
        Self::Send
    }
}

// Commander Error
#[derive(Debug, Error)]
pub enum CommunicateError {
    // should proxy
    #[error("should-proxy request receiver dropped")]
    ShouldProxy,
    #[error("h2 stream registration receiver dropped")]
    H2StreamRegister,
    // http log
    #[error("history log send")]
    WriteHistory(#[from] SendError<CommanderToHistory>),
    #[error("directory create: {0}")]
    CreateDirectory(#[from] std::io::Error),
    // Intercept
    #[error("interceptor message send")]
    InterceptorSend,

    // Common
    #[error("commander response send")]
    Send(#[from] SendError<CommanderResponse>),
    #[error("no response channel for {0}: {1}")]
    ResponseNoId(SoldierId, CommanderResponse),
    #[error("no commander entry for {0} ({1})")]
    NoId(SoldierId, &'static str),

    // History
    #[error("repeater forward")]
    ToRepeater(#[from] SendError<ForwardInfo>),

    #[error(transparent)]
    Config(#[from] ConfigError),
}
