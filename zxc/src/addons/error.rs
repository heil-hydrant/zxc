use thiserror::Error;

use crate::forward_info::ForwardInfo;
use crate::io::{DirError, UnixSockError};
use crate::run::boundary::IsUIError;

#[derive(Debug, Error)]
pub enum AddonError {
    #[error("no ui")]
    NoUI,
    #[error("ui error| {0}")]
    UI(#[from] UnixSockError),
    #[error("wrong msg| {0:?}")]
    WrongMsg(ForwardInfo),
    #[error("addon not found| {0}")]
    AddonNotFound(String),
    #[error("json error| {0:?}")]
    Serialize(#[from] serde_json::Error),
    #[error("dir error| {0:?}")]
    DirError(#[from] DirError),
}

impl IsUIError for AddonError {
    fn is_ui_error(&self) -> bool {
        matches!(self, AddonError::UI(_))
    }

    fn needs_flush(&self) -> bool {
        false
    }
}
