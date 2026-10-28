use std::fmt::{Debug, Display, Formatter};
use std::path::PathBuf;
use std::sync::Arc;

use thiserror::Error;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_rustls::TlsConnector;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::rustls::client::WebPkiServerVerifier;

use crate::commander::CertError;
use crate::history::CommanderToHistory;
use crate::interceptor::ResumeInfo;

pub enum CommanderResponse {
    ClientConfig(Arc<TlsConnector>),
    Verifier(Arc<WebPkiServerVerifier>),
    ServerConfig(Option<Arc<ServerConfig>>),
    NewCertificate(Result<Arc<ServerConfig>, CertError>),
    HttpLog(Option<(usize, PathBuf, Sender<CommanderToHistory>)>),
    Resume(Option<ResumeInfo>),
    WsProxyReply(bool),
    WsRegisterReply((Receiver<CommanderResponse>, Sender<CommanderToHistory>)),
    WsLog(usize),
    WsInterceptReply(bool),
    Drop,
}

impl CommanderResponse {
    pub fn is_drop_msg(&self) -> bool {
        matches!(self, CommanderResponse::Drop)
    }

    pub fn wreq_need_response(&self) -> bool {
        if let CommanderResponse::Resume(Some(info)) = self {
            return info.is_wreq() && info.need_response();
        }
        false
    }
}

impl Display for CommanderResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CommanderResponse::ClientConfig(_) => write!(f, "ClientConfig"),
            CommanderResponse::Verifier(_) => write!(f, "Verifier"),
            CommanderResponse::ServerConfig(_) => write!(f, "ServerConfig"),
            CommanderResponse::NewCertificate(_) => {
                write!(f, "NewCertificate")
            }
            CommanderResponse::HttpLog(_) => write!(f, "HttpLog"),
            CommanderResponse::Resume(_) => write!(f, "Resume"),
            CommanderResponse::WsProxyReply(_) => write!(f, "WsProxyReply"),
            CommanderResponse::WsRegisterReply(..) => {
                write!(f, "WsRegisterReply")
            }
            CommanderResponse::WsLog(_) => write!(f, "WsLog"),
            CommanderResponse::WsInterceptReply(_) => write!(f, "WsIntercept"),
            CommanderResponse::Drop => write!(f, "Drop"),
        }
    }
}

impl Debug for CommanderResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Debug, Error)]
#[error("wrong response| {0}")]
pub struct WrongMessage(String);

// Various TryFrom implementations to get associated values from
// CommanderResponse
impl TryFrom<CommanderResponse> for Arc<TlsConnector> {
    type Error = WrongMessage;

    fn try_from(value: CommanderResponse) -> Result<Self, Self::Error> {
        match value {
            CommanderResponse::ClientConfig(config) => Ok(config),
            _ => Err(WrongMessage(value.to_string())),
        }
    }
}

impl TryFrom<CommanderResponse> for Arc<WebPkiServerVerifier> {
    type Error = WrongMessage;

    fn try_from(value: CommanderResponse) -> Result<Self, Self::Error> {
        match value {
            CommanderResponse::Verifier(verifier) => Ok(verifier),
            _ => Err(WrongMessage(value.to_string())),
        }
    }
}

impl TryFrom<CommanderResponse> for Option<Arc<ServerConfig>> {
    type Error = WrongMessage;

    fn try_from(value: CommanderResponse) -> Result<Self, Self::Error> {
        match value {
            CommanderResponse::ServerConfig(config) => Ok(config),
            _ => Err(WrongMessage(value.to_string())),
        }
    }
}

impl TryFrom<CommanderResponse> for Result<Arc<ServerConfig>, CertError> {
    type Error = WrongMessage;

    fn try_from(value: CommanderResponse) -> Result<Self, Self::Error> {
        match value {
            CommanderResponse::NewCertificate(config) => Ok(config),
            _ => Err(WrongMessage(value.to_string())),
        }
    }
}

impl TryFrom<CommanderResponse>
    for Option<(usize, PathBuf, Sender<CommanderToHistory>)>
{
    type Error = WrongMessage;

    fn try_from(value: CommanderResponse) -> Result<Self, Self::Error> {
        match value {
            CommanderResponse::HttpLog(config) => Ok(config),
            _ => Err(WrongMessage(value.to_string())),
        }
    }
}

impl TryFrom<CommanderResponse> for Option<ResumeInfo> {
    type Error = WrongMessage;

    fn try_from(value: CommanderResponse) -> Result<Self, Self::Error> {
        match value {
            CommanderResponse::Resume(config) => Ok(config),
            _ => Err(WrongMessage(value.to_string())),
        }
    }
}

impl TryFrom<CommanderResponse>
    for (Receiver<CommanderResponse>, Sender<CommanderToHistory>)
{
    type Error = WrongMessage;

    fn try_from(value: CommanderResponse) -> Result<Self, Self::Error> {
        match value {
            CommanderResponse::WsRegisterReply(config) => Ok(config),
            _ => Err(WrongMessage(value.to_string())),
        }
    }
}

impl TryFrom<CommanderResponse> for Option<usize> {
    type Error = WrongMessage;

    fn try_from(value: CommanderResponse) -> Result<Self, Self::Error> {
        match value {
            CommanderResponse::WsLog(config) => Ok(Some(config)),
            _ => Err(WrongMessage(value.to_string())),
        }
    }
}

impl TryFrom<CommanderResponse> for bool {
    type Error = WrongMessage;

    fn try_from(value: CommanderResponse) -> Result<Self, Self::Error> {
        match value {
            CommanderResponse::WsProxyReply(config) => Ok(config),
            _ => Err(WrongMessage(value.to_string())),
        }
    }
}
