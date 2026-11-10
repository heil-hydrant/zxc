use bytes::BytesMut;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

use crate::commander::{CodecError, CommanderError};
use crate::interceptor::InterUIOps;
use crate::run::boundary::UiError;

#[derive(Debug, Error)]
pub enum InterceptorError {
    // ----- Commander -----
    #[error(transparent)]
    Commander(#[from] CommanderError),

    // ------ UI ------
    #[error("invalid interceptor message ({0}): {1}")]
    MsgDeserialize(String, #[source] serde_json::Error),

    // ----- codec_op ------
    #[error(transparent)]
    Codec(#[from] CodecError),
    #[error(transparent)]
    Ui(#[from] UiError),
}

crate::impl_ui_error!(InterceptorError);

impl From<SendError<InterUIOps>> for InterceptorError {
    fn from(_: SendError<InterUIOps>) -> Self {
        Self::Commander(CommanderError::Send)
    }
}

impl From<(&BytesMut, serde_json::Error)> for InterceptorError {
    fn from((val, e): (&BytesMut, serde_json::Error)) -> Self {
        InterceptorError::MsgDeserialize(
            String::from_utf8_lossy(val).to_string(),
            e,
        )
    }
}
