use bytes::BytesMut;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

use crate::commander::CodecError;
use crate::interceptor::InterUIOps;
use crate::io::UnixSockError;
use crate::run::boundary::IsUIError;

#[derive(Debug, Error)]
pub enum InterceptorError {
    // ----- Commander -----
    // commander send
    #[error("Message to Commander| {0}")]
    CommanderSend(#[from] SendError<InterUIOps>),

    // ------ UI ------
    // serde decode
    #[error("Message Encode| {0}")]
    MsgSerialize(#[from] serde_json::Error),
    #[error("Message Decode| {0}| {1}")]
    MsgDeserialize(String, serde_json::Error),

    #[error("UI Closed")]
    UIclosed,

    // ----- codec_op ------
    #[error("{0}")]
    Codec(#[from] CodecError),
    // unknown
    #[error("{0}")]
    UIError(#[from] UnixSockError),
    #[error("no ui")]
    NoUI,
}

impl From<(&BytesMut, serde_json::Error)> for InterceptorError {
    fn from((val, e): (&BytesMut, serde_json::Error)) -> Self {
        InterceptorError::MsgDeserialize(
            String::from_utf8_lossy(val).to_string(),
            e,
        )
    }
}

impl IsUIError for InterceptorError {
    fn is_ui_error(&self) -> bool {
        matches!(self, InterceptorError::UIError(_))
    }

    fn needs_flush(&self) -> bool {
        false
    }
}
