use thiserror::Error;

use crate::forward_info::ForwardInfo;
use crate::io::{DirError, FileErrorInfo};
use crate::run::boundary::UiError;

#[derive(Debug, Error)]
pub enum AddonError {
    #[error(transparent)]
    Ui(#[from] UiError),
    #[error("unexpected forward info: {0:?}")]
    WrongMsg(ForwardInfo),
    #[error("addon not found: {0}")]
    AddonNotFound(String),
    #[error("h2 request: {0}")]
    H2Request(String),
    #[error(transparent)]
    RequestFile(#[from] FileErrorInfo),
    #[error(transparent)]
    DirError(#[from] DirError),
}

crate::impl_ui_error!(AddonError);
