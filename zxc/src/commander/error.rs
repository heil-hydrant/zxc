use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

use super::CommanderResponse;
use crate::config::ConfigError;
use crate::forward_info::ForwardInfo;
use crate::history::CommanderToHistory;
use crate::id::SoldierId;

// Commander Error
#[derive(Debug, Error)]
pub enum CommunicateError {
    // should proxy
    #[error("Should Proxy")]
    ShouldProxy,
    #[error("H2 stream register")]
    H2StreamRegister,
    // http log
    #[error("write history")]
    WriteHistory(#[from] SendError<CommanderToHistory>),
    #[error("Unable to create directory| {0}")]
    CreateDirectory(#[from] std::io::Error),
    // Intercept
    #[error("Interceptor send")]
    InterceptorSend,

    // Common
    #[error("Send Error| {0}")]
    Send(#[from] SendError<CommanderResponse>),
    #[error("Noid| {0}| {1}")]
    ResponseNoId(SoldierId, CommanderResponse),
    #[error("NoId| {0}| {1}")]
    NoId(SoldierId, &'static str),

    // History
    #[error("History to repeater| {0}")]
    ToRepeater(#[from] SendError<ForwardInfo>),

    #[error("Config| {0}")]
    Config(#[from] ConfigError),
}
